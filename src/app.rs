use std::sync::atomic::{AtomicU32, Ordering};
use windows::Win32::Foundation::{BOOL, LPARAM};
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::AppConfig;
use crate::diagnostics::Diagnostics;
use crate::executor::execute_command;
use crate::geometry::{GeometryConfig, Point, Sector};
use crate::input::{
    InputManager, WM_WINPIE_ACTIVATE, WM_WINPIE_ALLKEYSUP, WM_WINPIE_LBUTTONDOWN,
    WM_WINPIE_MENU_ESC, WM_WINPIE_MENU_KEYDOWN, WM_WINPIE_MENU_KEYUP, WM_WINPIE_MENU_TAB,
    WM_WINPIE_MOUSEMOVE, WM_WINPIE_RBUTTONDOWN, WM_WINPIE_WINUP,
};
use crate::interaction::{InteractionEffect, InteractionEvent, InteractionFsm};
use crate::overlay::{ModalMenuOverlay, OverlayWindow, ToastOverlay};

static APP_THREAD_ID: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> BOOL {
    if ctrl_type == 0 || ctrl_type == 1 || ctrl_type == 2 {
        let tid = APP_THREAD_ID.load(Ordering::SeqCst);
        if tid != 0 {
            let _ = PostThreadMessageW(tid, WM_QUIT, windows::Win32::Foundation::WPARAM(0), LPARAM(0));
            return BOOL(1);
        }
    }
    BOOL(0)
}

pub struct Application {
    config: AppConfig,
    diagnostics: Diagnostics,
    fsm: InteractionFsm,
    overlay: OverlayWindow,
    modal_menu: ModalMenuOverlay,
    toast: ToastOverlay,
    hint_toast: ToastOverlay,
    _control_window: crate::ipc::ControlWindow,
    input_manager: InputManager,
    active_profile: Option<usize>,
}

impl Application {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let config = AppConfig::load_or_default("config/default.yaml");
        let diagnostics = Diagnostics::new(config.diagnostics.clone());

        let geom_config = GeometryConfig {
            radius: config.overlay.radius,
            deadzone: config.overlay.deadzone,
            rotation_degrees: config.wheel.rotation_degrees,
            slices: config.wheel.slices,
        };

        let fsm = InteractionFsm::new(geom_config);

        let hinstance = unsafe { GetModuleHandleW(None)?.into() };
        let overlay = OverlayWindow::new(&config, hinstance)?;
        let modal_menu = ModalMenuOverlay::new(hinstance)?;
        let toast = ToastOverlay::new(hinstance)?;
        let hint_toast = ToastOverlay::new(hinstance)?;
        let control_window = crate::ipc::ControlWindow::new(hinstance)?;

        let thread_id = unsafe { GetCurrentThreadId() };
        APP_THREAD_ID.store(thread_id, Ordering::SeqCst);
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(console_ctrl_handler), true);
        }

        let input_manager = InputManager::install(thread_id)?;

        Ok(Self {
            config,
            diagnostics,
            fsm,
            overlay,
            modal_menu,
            toast,
            hint_toast,
            _control_window: control_window,
            input_manager,
            active_profile: None,
        })
    }

    pub fn run(&mut self, is_daemon: bool) -> Result<(), Box<dyn std::error::Error>> {
        if !is_daemon {
            println!("WinPie running.");
            println!("- Press Win+Esc to open radial menu.");
            println!("- Left-click sector to commit or enter submenu.");
            println!("- Releasing Win key commits hovered sector.");
            println!("- Right-click or Esc cancels anytime.");
            println!("- In Modal Submenus: Hover or KeyDown previews, Click or KeyUp executes.");
            println!("- Run 'winpie help' in any terminal for CLI commands.");
            println!("- Press Ctrl+C in terminal or run 'winpie kill' to exit.\n");
        }

        // Display configurable startup toast
        if self.config.startup_toast.enabled && !self.config.startup_toast.text.is_empty() {
            let mut pt = windows::Win32::Foundation::POINT::default();
            let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) };
            let anchor = Point::new(pt.x, pt.y);
            let toast_cfg = crate::config::ToastConfig {
                enabled: true,
                show_hover_tooltips: true,
                duration_ms: self.config.startup_toast.duration_ms,
                corner: self.config.startup_toast.corner,
                margin_x: self.config.startup_toast.margin_x,
                margin_y: self.config.startup_toast.margin_y,
                font_size: self.config.startup_toast.font_size,
                corner_radius: self.config.startup_toast.corner_radius,
                show_sector_direction: false,
            };
            self.toast.show(anchor, &self.config.startup_toast.text, &toast_cfg, &self.config.theme);
        }

        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                match msg.message {
                    WM_WINPIE_ACTIVATE => {
                        let pt = Point::from_lparam(msg.lParam.0);
                        self.handle_activate(pt);
                    }
                    WM_WINPIE_MOUSEMOVE => {
                        let pt = Point::from_lparam(msg.lParam.0);
                        self.handle_mousemove(pt);
                    }
                    WM_WINPIE_LBUTTONDOWN => {
                        let pt = Point::from_lparam(msg.lParam.0);
                        self.handle_lbuttondown(pt);
                    }
                    WM_WINPIE_WINUP => {
                        let pt = Point::from_lparam(msg.lParam.0);
                        self.handle_winup(pt);
                    }
                    WM_WINPIE_RBUTTONDOWN => {
                        self.handle_rbuttondown();
                    }
                    WM_WINPIE_MENU_KEYDOWN => {
                        let ch = (msg.wParam.0 as u8) as char;
                        self.handle_menu_keydown(ch);
                    }
                    WM_WINPIE_MENU_KEYUP => {
                        let ch = (msg.wParam.0 as u8) as char;
                        self.handle_menu_keyup(ch);
                    }
                    WM_WINPIE_MENU_TAB => {
                        let shift = msg.wParam.0 != 0;
                        self.handle_menu_tab(shift);
                    }
                    WM_WINPIE_MENU_ESC => {
                        self.handle_menu_esc();
                    }
                    WM_WINPIE_ALLKEYSUP => {
                        self.handle_allkeysup();
                    }
                    WM_QUIT => {
                        break;
                    }
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }

        println!("[WinPie] Clean shutdown complete.");
        Ok(())
    }

    fn handle_activate(&mut self, anchor: Point) {
        // Detect foreground window context
        let ctx = crate::context::active_window_context();
        let profile_idx = self.config.find_matching_profile(&ctx);
        self.active_profile = profile_idx;

        let effect = self.fsm.transition(InteractionEvent::WinEscDown(anchor));
        if let InteractionEffect::Activated { anchor } = effect {
            self.diagnostics.log_activate(anchor);
            self.overlay.show_at(anchor, None, profile_idx);
        }
    }

    fn handle_mousemove(&mut self, pt: Point) {
        let theme = self.config.get_theme_for_profile(self.active_profile);

        if self.fsm.is_modal_menu() {
            let anchor = self.fsm.active_anchor().unwrap_or(pt);
            if let Some(nav_stack) = self.fsm.menu_nav_stack() {
                let hit = self.modal_menu.hit_test(pt, anchor, nav_stack);
                let hit_key = hit.as_ref().map(|(k, _)| *k);
                let effect = self.fsm.transition(InteractionEvent::MenuHover(hit_key));

                if let InteractionEffect::MenuHighlightChanged { key, item } = effect {
                    if let Some(stack) = self.fsm.menu_nav_stack() {
                        self.modal_menu.show(anchor, stack, key, theme);
                    }
                    if let Some(item) = item {
                        if let Some(tt) = &item.tooltip {
                            self.toast.show_tooltip(anchor, tt, &self.config.toast, theme);
                        } else {
                            self.toast.hide();
                        }
                    } else {
                        self.toast.hide();
                    }
                }
            }
            return;
        }

        let effect = self.fsm.transition(InteractionEvent::MouseMove(pt));
        if let InteractionEffect::HoverChanged { to, .. } = effect {
            self.diagnostics.log_hover(to);
            self.overlay.update_hover(to);

            // Display ambient tooltip toast if hovered sector has a tooltip configured
            if let Some(sector) = to {
                if let Some(tooltip) = self.config.get_tooltip_for_sector_with_profile(sector, self.active_profile) {
                    self.toast.show_tooltip(pt, tooltip, &self.config.toast, theme);
                } else {
                    self.toast.hide();
                }
            } else {
                self.toast.hide();
            }
        }
    }

    fn handle_commit_resolution(&mut self, anchor: Point, sector: Sector) {
        let profile_idx = self.active_profile;
        let theme = self.config.get_theme_for_profile(profile_idx);

        self.diagnostics.log_commit(sector);
        self.overlay.hide();
        self.toast.hide();

        // Check if this sector spawns a submenu
        if let Some(menu) = self.config.get_menu_for_sector_with_profile(sector, profile_idx).cloned() {
            self.input_manager.set_active(false);
            self.input_manager.set_modal_menu(true);

            let effect = self.fsm.transition(InteractionEvent::MenuSpawn {
                anchor,
                menu: menu.clone(),
            });

            if let InteractionEffect::MenuSpawned { anchor, menu } = effect {
                self.modal_menu.show(anchor, &[menu], None, theme);
                self.hint_toast.show_hint(anchor, &self.config.hint_toast, theme);
            }
            return;
        }

        // Otherwise direct command execution
        self.input_manager.set_active(false);

        let label = self.config.get_label_for_sector_with_profile(sector, profile_idx);
        let text = if self.config.toast.show_sector_direction {
            format!("Committed: {} ({})", label, sector.name())
        } else {
            format!("Committed: {}", label)
        };
        self.toast.show(anchor, &text, &self.config.toast, theme);

        if let Some(cmd) = self.config.get_command_for_sector_with_profile(sector, profile_idx) {
            match execute_command(cmd) {
                Ok(_) => println!("[WinPie:ACTION] Executed: {}", cmd),
                Err(e) => eprintln!("[WinPie:ACTION ERROR] Failed to execute '{}': {}", cmd, e),
            }
        }
    }

    fn handle_lbuttondown(&mut self, pt: Point) {
        let theme = self.config.get_theme_for_profile(self.active_profile);
        if self.fsm.is_modal_menu() {
            let anchor = self.fsm.active_anchor().unwrap_or(pt);
            if let Some(nav_stack) = self.fsm.menu_nav_stack() {
                let hit = self.modal_menu.hit_test(pt, anchor, nav_stack);
                if let Some((k, _)) = hit {
                    let keys_held = InputManager::are_keys_held();
                    let effect = self.fsm.transition(InteractionEvent::MenuSelect(k, keys_held));
                    match effect {
                        InteractionEffect::MenuExecuted { command, label } => {
                            self.modal_menu.hide();
                            self.hint_toast.hide();
                            self.toast.hide();
                            self.input_manager.set_modal_menu(false);

                            let text = format!("Executed: {}", label);
                            self.toast.show(anchor, &text, &self.config.toast, theme);

                            match execute_command(&command) {
                                Ok(_) => println!("[WinPie:ACTION] Executed: {}", command),
                                Err(e) => eprintln!("[WinPie:ACTION ERROR] Failed to execute '{}': {}", command, e),
                            }
                        }
                        InteractionEffect::MenuDrillDown { .. } => {
                            self.toast.hide();
                            if let Some(stack) = self.fsm.menu_nav_stack() {
                                self.modal_menu.show(anchor, stack, None, theme);
                            }
                        }
                        _ => {}
                    }
                    return;
                }
            }
        }

        let anchor = self.fsm.active_anchor().unwrap_or(pt);
        let keys_held = InputManager::are_keys_held();
        let effect = self.fsm.transition(InteractionEvent::LButtonDown(pt, keys_held));
        match effect {
            InteractionEffect::Committed(sector) => {
                self.handle_commit_resolution(anchor, sector);
            }
            InteractionEffect::Cancelled => {
                self.diagnostics.log_cancel();
                self.overlay.hide();
                self.toast.hide();
                self.input_manager.set_active(false);
            }
            _ => {}
        }
    }

    fn handle_winup(&mut self, pt: Point) {
        let anchor = self.fsm.active_anchor().unwrap_or(pt);
        let keys_held = InputManager::are_keys_held();
        let effect = self.fsm.transition(InteractionEvent::WinUp(pt, keys_held));
        match effect {
            InteractionEffect::Committed(sector) => {
                self.handle_commit_resolution(anchor, sector);
            }
            InteractionEffect::Cancelled => {
                self.diagnostics.log_cancel();
                self.overlay.hide();
                self.toast.hide();
                self.input_manager.set_active(false);
            }
            _ => {}
        }
    }

    fn handle_rbuttondown(&mut self) {
        let keys_held = InputManager::are_keys_held();
        let _ = self.fsm.transition(InteractionEvent::RButtonDown(Point::default(), keys_held));
        self.diagnostics.log_cancel();
        self.overlay.hide();
        self.modal_menu.hide();
        self.hint_toast.hide();
        self.toast.hide();
        self.input_manager.set_active(false);
        self.input_manager.set_modal_menu(false);
    }

    fn handle_menu_keydown(&mut self, ch: char) {
        let theme = self.config.get_theme_for_profile(self.active_profile);
        let effect = self.fsm.transition(InteractionEvent::MenuKeyDown(ch));
        if let InteractionEffect::MenuHighlightChanged { key, item } = effect {
            if let Some(anchor) = self.fsm.active_anchor() {
                if let Some(nav_stack) = self.fsm.menu_nav_stack() {
                    self.modal_menu.show(anchor, nav_stack, key, theme);
                }

                if let Some(item) = item {
                    if let Some(tt) = &item.tooltip {
                        self.toast.show_tooltip(anchor, tt, &self.config.toast, theme);
                    } else {
                        self.toast.hide();
                    }
                } else {
                    self.toast.hide();
                }
            }
        }
    }

    fn handle_menu_keyup(&mut self, ch: char) {
        let theme = self.config.get_theme_for_profile(self.active_profile);
        let keys_held = InputManager::are_keys_held();
        let effect = self.fsm.transition(InteractionEvent::MenuKeyUp(ch, keys_held));
        let anchor = self.fsm.active_anchor().unwrap_or_default();

        match effect {
            InteractionEffect::MenuExecuted { command, label } => {
                self.modal_menu.hide();
                self.hint_toast.hide();
                self.toast.hide();
                self.input_manager.set_modal_menu(false);

                let text = format!("Executed: {}", label);
                self.toast.show(anchor, &text, &self.config.toast, theme);

                match execute_command(&command) {
                    Ok(_) => println!("[WinPie:ACTION] Executed: {}", command),
                    Err(e) => eprintln!("[WinPie:ACTION ERROR] Failed to execute '{}': {}", command, e),
                }
            }
            InteractionEffect::MenuDrillDown { .. } => {
                self.toast.hide();
                if let Some(nav_stack) = self.fsm.menu_nav_stack() {
                    self.modal_menu.show(anchor, nav_stack, None, theme);
                }
            }
            InteractionEffect::MenuHighlightChanged { key, .. } => {
                self.toast.hide();
                if let Some(nav_stack) = self.fsm.menu_nav_stack() {
                    self.modal_menu.show(anchor, nav_stack, key, theme);
                }
            }
            _ => {}
        }
    }

    fn handle_menu_tab(&mut self, shift: bool) {
        let theme = self.config.get_theme_for_profile(self.active_profile);
        let effect = self.fsm.transition(InteractionEvent::MenuTab { shift });
        let anchor = self.fsm.active_anchor().unwrap_or_default();
        self.toast.hide();

        match effect {
            InteractionEffect::MenuBacktracked { .. } | InteractionEffect::MenuResetToRoot { .. } => {
                if let Some(nav_stack) = self.fsm.menu_nav_stack() {
                    self.modal_menu.show(anchor, nav_stack, None, theme);
                }
            }
            _ => {}
        }
    }

    fn handle_menu_esc(&mut self) {
        let keys_held = InputManager::are_keys_held();
        let _ = self.fsm.transition(InteractionEvent::MenuEsc(keys_held));
        self.modal_menu.hide();
        self.hint_toast.hide();
        self.toast.hide();
        self.input_manager.set_modal_menu(false);
    }

    fn handle_allkeysup(&mut self) {
        let _ = self.fsm.transition(InteractionEvent::AllKeysUp);
    }
}
