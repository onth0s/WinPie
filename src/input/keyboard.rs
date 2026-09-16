use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::geometry::Point;
use super::state::*;
use super::{WM_WINPIE_ACTIVATE, WM_WINPIE_ALLKEYSUP, WM_WINPIE_WINUP};

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

        // Clear WAIT_RELEASE once both Win and Esc have been physically released
        if is_up
            && !win_held
            && !ESCAPE_DOWN.load(Ordering::SeqCst)
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

        // Activation check: Win held + Escape DOWN
        if is_down && vk == VK_ESCAPE && win_held {
            let was_active = IS_ACTIVE.load(Ordering::SeqCst);
            let blocked = REQUIRE_KEY_RELEASE.load(Ordering::SeqCst);

            if !was_active && !blocked {
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
            } else if was_active || blocked {
                return LRESULT(1);
            }
        }

        // On Win key release while active: trigger commit or cancel check
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
