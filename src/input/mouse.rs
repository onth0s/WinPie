use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::geometry::Point;
use super::state::*;
use super::{WM_WINPIE_LBUTTONDOWN, WM_WINPIE_MOUSEMOVE, WM_WINPIE_RBUTTONDOWN};

/// Low-level mouse hook callback procedure.
///
/// # Safety
/// Called by the Windows hook manager with raw pointer arguments.
/// `lparam` must point to a valid `MSLLHOOKSTRUCT` when `n_code >= 0`.
pub unsafe extern "system" fn ll_mouse_proc(
    n_code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let mouse = *(lparam.0 as *const MSLLHOOKSTRUCT);
        let msg = wparam.0 as u32;

        let active = IS_ACTIVE.load(Ordering::SeqCst);
        let modal_active = IS_MODAL_MENU.load(Ordering::SeqCst);

        if active || modal_active {
            match msg {
                WM_MOUSEMOVE => {
                    let cur_x = mouse.pt.x;
                    let cur_y = mouse.pt.y;
                    let prev_x = LAST_POSTED_X.load(Ordering::Relaxed);
                    let prev_y = LAST_POSTED_Y.load(Ordering::Relaxed);

                    // Deduplicate identical points to keep hook returning in <1µs
                    if cur_x != prev_x || cur_y != prev_y {
                        LAST_POSTED_X.store(cur_x, Ordering::Relaxed);
                        LAST_POSTED_Y.store(cur_y, Ordering::Relaxed);

                        let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                        if tid != 0 {
                            let _ = PostThreadMessageW(
                                tid,
                                WM_WINPIE_MOUSEMOVE,
                                WPARAM(0),
                                LPARAM(Point::new(cur_x, cur_y).to_lparam()),
                            );
                        }
                    }
                }
                WM_LBUTTONDOWN => {
                    let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                    if tid != 0 {
                        let _ = PostThreadMessageW(
                            tid,
                            WM_WINPIE_LBUTTONDOWN,
                            WPARAM(0),
                            LPARAM(Point::new(mouse.pt.x, mouse.pt.y).to_lparam()),
                        );
                    }
                    return LRESULT(1);
                }
                WM_RBUTTONDOWN => {
                    set_active(false);
                    set_modal_menu(false);

                    let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                    if tid != 0 {
                        let _ = PostThreadMessageW(
                            tid,
                            WM_WINPIE_RBUTTONDOWN,
                            WPARAM(0),
                            LPARAM(Point::new(mouse.pt.x, mouse.pt.y).to_lparam()),
                        );
                    }
                    return LRESULT(1);
                }
                WM_LBUTTONUP | WM_RBUTTONUP => {
                    // Let mouse up pass through to maintain Windows input integrity
                }
                _ => {}
            }
        }
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}
