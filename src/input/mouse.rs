use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::geometry::Point;
use super::state::*;
use super::{WM_WINPIE_LBUTTONDOWN, WM_WINPIE_MOUSEMOVE, WM_WINPIE_RBUTTONDOWN};

/// Pure right-button-up swallow decision.
///
/// Returns `true` when the up must be suppressed: a menu is live, or a previously
/// swallowed `WM_RBUTTONDOWN` still owes this up (`pair_owed` is set).
///
/// CRITICAL INVARIANT: this must be evaluated independently of any
/// `IS_ACTIVE` / `IS_MODAL_MENU` gating. The `WM_RBUTTONDOWN` handler clears both
/// flags in the *same callback* that swallows the down, so the matching
/// `WM_RBUTTONUP` routinely arrives with both flags already `false` while
/// `pair_owed` is still `true`. A pairing check gated on the flags (as it once was)
/// is dead code: the orphaned up then passes through `CallNextHookEx` into the app
/// behind the overlay, whose `DefWindowProc` converts it into `WM_CONTEXTMENU`.
fn should_swallow_rbutton_up(active: bool, modal_active: bool, pair_owed: bool) -> bool {
    active || modal_active || pair_owed
}

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

        // Right-button pairing runs OUTSIDE the active gate below: the down handler
        // clears IS_ACTIVE / IS_MODAL_MENU in the same callback that swallows the
        // down, so at up time those flags are already false. Only this
        // unconditional check can still suppress the orphaned up that would
        // otherwise reach the app behind and trigger its context menu.
        if msg == WM_RBUTTONUP
            && should_swallow_rbutton_up(
                active,
                modal_active,
                SWALLOW_RMB_UP.swap(false, Ordering::SeqCst),
            )
        {
            return LRESULT(1);
        }

        // Self-heal: a fresh, un-swallowed press invalidates any stale pair debt
        // from a missed up, so a later real right-click can never be half-eaten.
        if msg == WM_RBUTTONDOWN && !(active || modal_active) {
            SWALLOW_RMB_UP.store(false, Ordering::SeqCst);
        }

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
                    // Persist the swallow intent beyond this callback: IS_ACTIVE and
                    // IS_MODAL_MENU are cleared immediately below, so the physical
                    // WM_RBUTTONUP arrives with both gates already false. The
                    // top-level pairing check above then suppresses it, so the app
                    // behind never receives an orphaned up (which DefWindowProc
                    // would turn into WM_CONTEXTMENU).
                    SWALLOW_RMB_UP.store(true, Ordering::SeqCst);
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
                WM_LBUTTONUP => {
                    // Let left-button up pass through unchanged. An orphaned left-up
                    // is harmless to the background app; swallowing it could break
                    // gesture releases that began before activation.
                }
                _ => {}
            }
        }
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::should_swallow_rbutton_up;

    #[test]
    fn rbutton_up_swallow_when_pair_owed_while_inactive() {
        // The exact historical leak: menu already inactive when the up arrives,
        // but a swallowed down still owes this up -> must swallow (NOT gated on
        // the active flags).
        assert!(should_swallow_rbutton_up(false, false, true));
    }

    #[test]
    fn rbutton_up_swallow_while_menu_live() {
        assert!(should_swallow_rbutton_up(true, false, false));
        assert!(should_swallow_rbutton_up(false, true, false));
    }

    #[test]
    fn rbutton_up_passes_through_when_idle() {
        assert!(!should_swallow_rbutton_up(false, false, false));
    }
}