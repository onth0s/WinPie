use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::geometry::Point;
use super::state::*;
use super::{
    WM_WINPIE_ACTIVATE, WM_WINPIE_ALLKEYSUP, WM_WINPIE_MENU_ESC, WM_WINPIE_MENU_KEYDOWN,
    WM_WINPIE_MENU_KEYUP, WM_WINPIE_MENU_TAB, WM_WINPIE_WINUP,
};

/// Low-level keyboard hook callback procedure.
///
/// # Safety
/// Called by the Windows hook manager with raw pointer arguments.
/// `lparam` must point to a valid `KBDLLHOOKSTRUCT` when `n_code >= 0`.
pub unsafe extern "system" fn ll_keyboard_proc(
    n_code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);

        // Pass through synthetic Start-menu disarm mask key (0xE8)
        if kbd.vkCode == 0xE8 {
            return CallNextHookEx(None, n_code, wparam, lparam);
        }

        let msg = wparam.0 as u32;
        let vk = VIRTUAL_KEY(kbd.vkCode as u16);

        let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

        // Track Windows modifier keys
        if vk == VK_LWIN {
            LEFT_WIN_DOWN.store(is_down, Ordering::SeqCst);
        } else if vk == VK_RWIN {
            RIGHT_WIN_DOWN.store(is_down, Ordering::SeqCst);
        } else if vk == VK_ESCAPE {
            ESCAPE_DOWN.store(is_down, Ordering::SeqCst);
        } else if vk == VK_SHIFT || vk == VK_LSHIFT || vk == VK_RSHIFT {
            SHIFT_DOWN.store(is_down, Ordering::SeqCst);
        }

        let win_held = is_win_down();
        let esc_held = is_escape_down();

        // Clear WAIT_RELEASE once all activation keys have been physically released
        if is_up
            && !win_held
            && !esc_held
            && REQUIRE_KEY_RELEASE.swap(false, Ordering::SeqCst)
        {
            let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
            if tid != 0 {
                let _ = PostThreadMessageW(
                    tid,
                    WM_WINPIE_ALLKEYSUP,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }

        let modal_active = IS_MODAL_MENU.load(Ordering::SeqCst);

        // 1. Modal Menu active input handling
        if modal_active {
            let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);

            if vk == VK_ESCAPE && is_down {
                if tid != 0 {
                    let _ = PostThreadMessageW(
                        tid,
                        WM_WINPIE_MENU_ESC,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
                return LRESULT(1);
            }

            if vk == VK_TAB && is_down {
                let shift = SHIFT_DOWN.load(Ordering::SeqCst);
                if tid != 0 {
                    let _ = PostThreadMessageW(
                        tid,
                        WM_WINPIE_MENU_TAB,
                        WPARAM(if shift { 1 } else { 0 }),
                        LPARAM(0),
                    );
                }
                return LRESULT(1);
            }

            // Alphanumeric shortcut keys (A-Z, 0-9)
            if (kbd.vkCode >= 0x41 && kbd.vkCode <= 0x5A) || (kbd.vkCode >= 0x30 && kbd.vkCode <= 0x39) {
                let ch = ((kbd.vkCode as u8) as char).to_ascii_lowercase();
                if tid != 0 {
                    let post_msg = if is_down {
                        WM_WINPIE_MENU_KEYDOWN
                    } else {
                        WM_WINPIE_MENU_KEYUP
                    };
                    let _ = PostThreadMessageW(
                        tid,
                        post_msg,
                        WPARAM(ch as usize),
                        LPARAM(0),
                    );
                }
                return LRESULT(1);
            }

            // Allow all other keys (like Alt, Ctrl, Enter, Letters not in menu) to pass through naturally
            // to avoid ever locking the system
        }

        // 2. Radial Menu activation check: Win held + Escape DOWN
        if is_down && vk == VK_ESCAPE && win_held {
            let was_active = IS_ACTIVE.load(Ordering::SeqCst);
            let require_release = REQUIRE_KEY_RELEASE.load(Ordering::SeqCst);

            if !was_active && !modal_active && !require_release {
                // Reset mouse deduplication
                reset_dedup();

                IS_ACTIVE.store(true, Ordering::SeqCst);

                // Disarm Windows Start menu trigger cleanly
                disarm_windows_key();

                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);

                let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                if tid != 0 {
                    let _ = PostThreadMessageW(
                        tid,
                        WM_WINPIE_ACTIVATE,
                        WPARAM(0),
                        LPARAM(Point::new(pt.x, pt.y).to_lparam()),
                    );
                }
                return LRESULT(1);
            } else if was_active || modal_active || require_release {
                return LRESULT(1);
            }
        }

        // 3. On Win key release while radial wheel active: trigger commit or cancel check
        if is_up && (vk == VK_LWIN || vk == VK_RWIN) {
            let was_active = IS_ACTIVE.load(Ordering::SeqCst);
            if was_active && !win_held {
                IS_ACTIVE.store(false, Ordering::SeqCst);

                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);

                let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                if tid != 0 {
                    let _ = PostThreadMessageW(
                        tid,
                        WM_WINPIE_WINUP,
                        WPARAM(0),
                        LPARAM(Point::new(pt.x, pt.y).to_lparam()),
                    );
                }
            }
        }
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}
