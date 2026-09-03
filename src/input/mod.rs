use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// Custom thread messages sent from hooks to the main message loop
pub const WM_WINPIE_ACTIVATE: u32 = WM_USER + 101;
pub const WM_WINPIE_MOUSEMOVE: u32 = WM_USER + 102;
pub const WM_WINPIE_LBUTTONDOWN: u32 = WM_USER + 103;
pub const WM_WINPIE_RBUTTONDOWN: u32 = WM_USER + 104;

static MAIN_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// Tracks low-level key state
static LEFT_WIN_DOWN: AtomicBool = AtomicBool::new(false);
static RIGHT_WIN_DOWN: AtomicBool = AtomicBool::new(false);
static ESCAPE_DOWN: AtomicBool = AtomicBool::new(false);
static IS_ACTIVE: AtomicBool = AtomicBool::new(false);

// When cancelled/resolved while activation keys are still held, arm re-press requirement.
// WinPie will NOT re-activate until keys are released and pressed again.
static REQUIRE_KEY_RELEASE: AtomicBool = AtomicBool::new(false);

pub struct InputManager {
    kbd_hook: HHOOK,
    mouse_hook: HHOOK,
}

impl InputManager {
    pub fn install(thread_id: u32) -> Result<Self, windows::core::Error> {
        MAIN_THREAD_ID.store(thread_id, Ordering::SeqCst);

        unsafe {
            let kbd_hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(ll_keyboard_proc),
                HMODULE::default(),
                0,
            )?;

            let mouse_hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(ll_mouse_proc),
                HMODULE::default(),
                0,
            )?;

            Ok(Self {
                kbd_hook,
                mouse_hook,
            })
        }
    }

    pub fn set_active(&self, active: bool) {
        IS_ACTIVE.store(active, Ordering::SeqCst);
        if !active {
            // If Win or Escape is still held when returning to idle, require full release before next activation
            let win_held = LEFT_WIN_DOWN.load(Ordering::SeqCst) || RIGHT_WIN_DOWN.load(Ordering::SeqCst);
            let esc_held = ESCAPE_DOWN.load(Ordering::SeqCst);
            if win_held || esc_held {
                REQUIRE_KEY_RELEASE.store(true, Ordering::SeqCst);
            }
        }
    }
}

impl Drop for InputManager {
    fn drop(&mut self) {
        unsafe {
            if !self.kbd_hook.0.is_null() {
                let _ = UnhookWindowsHookEx(self.kbd_hook);
            }
            if !self.mouse_hook.0.is_null() {
                let _ = UnhookWindowsHookEx(self.mouse_hook);
            }
        }
    }
}

/// Injects a dummy key event (Ctrl down + up) to disarm Windows shell Start menu activation
/// without leaving any modifier or key down in the system.
unsafe fn disarm_windows_key() {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

unsafe extern "system" fn ll_keyboard_proc(
    n_code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let msg = wparam.0 as u32;
        let vk = VIRTUAL_KEY(kbd.vkCode as u16);

        let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

        // Accurately track Windows modifier keys
        if vk == VK_LWIN {
            LEFT_WIN_DOWN.store(is_down, Ordering::SeqCst);
        } else if vk == VK_RWIN {
            RIGHT_WIN_DOWN.store(is_down, Ordering::SeqCst);
        } else if vk == VK_ESCAPE {
            ESCAPE_DOWN.store(is_down, Ordering::SeqCst);
        }

        let win_held = LEFT_WIN_DOWN.load(Ordering::SeqCst) || RIGHT_WIN_DOWN.load(Ordering::SeqCst);

        // Clear re-press requirement once both Win and Esc have been released
        if is_up {
            if !win_held && !ESCAPE_DOWN.load(Ordering::SeqCst) {
                REQUIRE_KEY_RELEASE.store(false, Ordering::SeqCst);
            }
        }

        // Activation check: Win held + Escape DOWN
        if is_down && vk == VK_ESCAPE && win_held {
            let was_active = IS_ACTIVE.load(Ordering::SeqCst);
            let blocked = REQUIRE_KEY_RELEASE.load(Ordering::SeqCst);

            if !was_active && !blocked {
                // Mark active immediately so mouse hook consumes instantly without gap (INV-INPUT-006)
                IS_ACTIVE.store(true, Ordering::SeqCst);

                // Disarm Windows Start menu trigger cleanly by notifying the OS that another key accompanied Win
                disarm_windows_key();

                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);

                let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                if tid != 0 {
                    let _ = PostThreadMessageW(
                        tid,
                        WM_WINPIE_ACTIVATE,
                        WPARAM(0),
                        LPARAM(((pt.y as isize) << 32) | (pt.x as u32 as isize)),
                    );
                }
                // Swallow qualifying trigger
                return LRESULT(1);
            } else if was_active || blocked {
                // When already active or silently waiting for release, swallow Escape to avoid shell leakage
                return LRESULT(1);
            }
        }

        // NEVER swallow Win key up or down events. Let the OS maintain its exact physical keyboard state.
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}

unsafe extern "system" fn ll_mouse_proc(
    n_code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let mouse = *(lparam.0 as *const MSLLHOOKSTRUCT);
        let msg = wparam.0 as u32;

        let active = IS_ACTIVE.load(Ordering::SeqCst);

        if active {
            match msg {
                WM_MOUSEMOVE => {
                    // Section 11 & INV-INPUT-003: WinPie observes mouse movement globally,
                    // but does NOT swallow WM_MOUSEMOVE (normal pointer movement continues).
                    let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                    if tid != 0 {
                        let _ = PostThreadMessageW(
                            tid,
                            WM_WINPIE_MOUSEMOVE,
                            WPARAM(0),
                            LPARAM(((mouse.pt.y as isize) << 32) | (mouse.pt.x as u32 as isize)),
                        );
                    }
                }
                WM_LBUTTONDOWN => {
                    // Section 12 & INV-INPUT-004:
                    // Must be consumed BEFORE foreground application delivery. Return 1 immediately!
                    let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                    if tid != 0 {
                        let _ = PostThreadMessageW(
                            tid,
                            WM_WINPIE_LBUTTONDOWN,
                            WPARAM(0),
                            LPARAM(((mouse.pt.y as isize) << 32) | (mouse.pt.x as u32 as isize)),
                        );
                    }
                    return LRESULT(1);
                }
                WM_RBUTTONDOWN => {
                    // Section 12 & 21: Right click is unconditional cancellation, swallowed immediately.
                    // Immediately mark inactive
                    IS_ACTIVE.store(false, Ordering::SeqCst);

                    // Require keys to be released before next activation
                    let win_held = LEFT_WIN_DOWN.load(Ordering::SeqCst) || RIGHT_WIN_DOWN.load(Ordering::SeqCst);
                    let esc_held = ESCAPE_DOWN.load(Ordering::SeqCst);
                    if win_held || esc_held {
                        REQUIRE_KEY_RELEASE.store(true, Ordering::SeqCst);
                    }

                    let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                    if tid != 0 {
                        let _ = PostThreadMessageW(
                            tid,
                            WM_WINPIE_RBUTTONDOWN,
                            WPARAM(0),
                            LPARAM(((mouse.pt.y as isize) << 32) | (mouse.pt.x as u32 as isize)),
                        );
                    }
                    return LRESULT(1);
                }
                WM_LBUTTONUP | WM_RBUTTONUP => {
                    // Swallow button release for the click that resolved/cancelled WinPie
                    return LRESULT(1);
                }
                _ => {}
            }
        }
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}
