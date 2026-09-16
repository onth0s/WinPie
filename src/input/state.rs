use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

pub static MAIN_THREAD_ID: AtomicU32 = AtomicU32::new(0);

// Tracks low-level key state
pub static LEFT_WIN_DOWN: AtomicBool = AtomicBool::new(false);
pub static RIGHT_WIN_DOWN: AtomicBool = AtomicBool::new(false);
pub static ESCAPE_DOWN: AtomicBool = AtomicBool::new(false);
pub static IS_ACTIVE: AtomicBool = AtomicBool::new(false);

// Deduplicate mouse movement in the hook to avoid flooding the message queue
pub static LAST_POSTED_X: AtomicI32 = AtomicI32::new(i32::MIN);
pub static LAST_POSTED_Y: AtomicI32 = AtomicI32::new(i32::MIN);

// WAIT_RELEASE state: WinPie will NOT re-activate until keys are released and pressed again.
pub static REQUIRE_KEY_RELEASE: AtomicBool = AtomicBool::new(false);

pub fn are_keys_held() -> bool {
    let win_held = LEFT_WIN_DOWN.load(Ordering::SeqCst) || RIGHT_WIN_DOWN.load(Ordering::SeqCst);
    let esc_held = ESCAPE_DOWN.load(Ordering::SeqCst);
    win_held || esc_held
}

pub fn set_active(active: bool) {
    IS_ACTIVE.store(active, Ordering::SeqCst);
    if !active && are_keys_held() {
        REQUIRE_KEY_RELEASE.store(true, Ordering::SeqCst);
    }
}

pub fn reset_dedup() {
    LAST_POSTED_X.store(i32::MIN, Ordering::SeqCst);
    LAST_POSTED_Y.store(i32::MIN, Ordering::SeqCst);
}

/// Injects a dummy key event (Ctrl down + up) to disarm Windows shell Start menu activation
/// without leaving any modifier or key down in the system.
///
/// # Safety
/// Calls Windows Win32 `SendInput` API to simulate low-level key strokes.
pub unsafe fn disarm_windows_key() {
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
