use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::diagnostics::{AppConfig, Diagnostics};
use crate::geometry::{GeometryConfig, Point};
use crate::input::{
    InputManager, WM_WINPIE_ACTIVATE, WM_WINPIE_LBUTTONDOWN, WM_WINPIE_MOUSEMOVE,
    WM_WINPIE_RBUTTONDOWN, WM_WINPIE_WINUP,
};
use crate::interaction::{InteractionEffect, InteractionEvent, InteractionFsm};
use crate::overlay::OverlayWindow;

pub struct Application {
    #[allow(dead_code)]
    config: AppConfig,
    diagnostics: Diagnostics,
    fsm: InteractionFsm,
    overlay: OverlayWindow,
    input_manager: InputManager,
}

impl Application {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Section 6: DPI awareness for physical screen coordinates
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

        let thread_id = unsafe { GetCurrentThreadId() };
        let input_manager = InputManager::install(thread_id)?;

        Ok(Self {
            config,
            diagnostics,
            fsm,
            overlay,
            input_manager,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("WinPie running.");
        println!("- Press Win+Esc to open radial menu.");
        println!("- Left-click sector to commit (click in deadzone or out-of-bounds to cancel).");
        println!("- Releasing Win key commits hovered sector (or cancels if in deadzone/out-of-bounds).");
        println!("- Right-click cancels anytime.");
        println!("- Press Ctrl+C in terminal to exit.\n");

        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                match msg.message {
                    WM_WINPIE_ACTIVATE => {
                        let pt = parse_lparam_point(msg.lParam);
                        self.handle_activate(pt);
                    }
                    WM_WINPIE_MOUSEMOVE => {
                        let pt = parse_lparam_point(msg.lParam);
                        self.handle_mousemove(pt);
                    }
                    WM_WINPIE_LBUTTONDOWN => {
                        let pt = parse_lparam_point(msg.lParam);
                        self.handle_lbuttondown(pt);
                    }
                    WM_WINPIE_WINUP => {
                        let pt = parse_lparam_point(msg.lParam);
                        self.handle_winup(pt);
                    }
                    WM_WINPIE_RBUTTONDOWN => {
                        self.handle_rbuttondown();
                    }
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_activate(&mut self, anchor: Point) {
        let effect = self.fsm.transition(InteractionEvent::WinEscDown(anchor));
        if let InteractionEffect::Activated { anchor } = effect {
            self.diagnostics.log_activate(anchor);
            self.overlay.show_at(anchor, None);
        }
    }

    fn handle_mousemove(&mut self, pt: Point) {
        let effect = self.fsm.transition(InteractionEvent::MouseMove(pt));
        if let InteractionEffect::HoverChanged { to, .. } = effect {
            self.diagnostics.log_hover(to);
            self.overlay.update_hover(to);
        }
    }

    fn handle_lbuttondown(&mut self, pt: Point) {
        let effect = self.fsm.transition(InteractionEvent::LButtonDown(pt));
        match effect {
            InteractionEffect::Committed(sector) => {
                self.diagnostics.log_commit(sector);
                self.overlay.hide();
                self.input_manager.set_active(false);
            }
            InteractionEffect::NoOpInDeadzone => {
                self.input_manager.set_active(true);
            }
            InteractionEffect::Cancelled => {
                self.diagnostics.log_cancel();
                self.overlay.hide();
                self.input_manager.set_active(false);
            }
            _ => {}
        }
    }

    fn handle_winup(&mut self, pt: Point) {
        let effect = self.fsm.transition(InteractionEvent::WinUp(pt));
        match effect {
            InteractionEffect::Committed(sector) => {
                self.diagnostics.log_commit(sector);
                self.overlay.hide();
                self.input_manager.set_active(false);
            }
            InteractionEffect::Cancelled => {
                self.diagnostics.log_cancel();
                self.overlay.hide();
                self.input_manager.set_active(false);
            }
            _ => {}
        }
    }

    fn handle_rbuttondown(&mut self) {
        let _ = self.fsm.transition(InteractionEvent::RButtonDown(Point::default()));
        self.diagnostics.log_cancel();
        self.overlay.hide();
        self.input_manager.set_active(false);
    }
}

fn parse_lparam_point(lparam: LPARAM) -> Point {
    let raw = lparam.0 as usize;
    let x = (raw & 0xFFFFFFFF) as u32 as i32;
    let y = ((raw >> 32) & 0xFFFFFFFF) as u32 as i32;
    Point::new(x, y)
}
