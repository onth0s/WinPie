pub mod keyboard;
pub mod mouse;
pub mod state;

use std::sync::atomic::Ordering;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::UI::WindowsAndMessaging::*;

use self::keyboard::ll_keyboard_proc;
use self::mouse::ll_mouse_proc;
pub use self::state::{are_keys_held, MAIN_THREAD_ID};

// Custom thread messages sent from hooks to the main message loop
pub const WM_WINPIE_ACTIVATE: u32 = WM_USER + 101;
pub const WM_WINPIE_MOUSEMOVE: u32 = WM_USER + 102;
pub const WM_WINPIE_LBUTTONDOWN: u32 = WM_USER + 103;
pub const WM_WINPIE_RBUTTONDOWN: u32 = WM_USER + 104;
pub const WM_WINPIE_WINUP: u32 = WM_USER + 105;
pub const WM_WINPIE_ALLKEYSUP: u32 = WM_USER + 106;

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

    pub fn are_keys_held() -> bool {
        state::are_keys_held()
    }

    pub fn set_active(&self, active: bool) {
        state::set_active(active);
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
