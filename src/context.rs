use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, POINT, RECT};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Performance::QueryPerformanceCounter;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{
    OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
};

use crate::geometry::Point;

static INVOCATION_SEQ: AtomicU64 = AtomicU64::new(1);

const CF_TEXT_VAL: u32 = 1;
const CF_UNICODETEXT_VAL: u32 = 13;
const CF_HDROP_VAL: u32 = 15;

/// Snapshot of the active foreground window context (backward-compatible view).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowContext {
    /// Lowercase executable name, e.g. "code.exe", "explorer.exe", "sublime_text.exe"
    pub process_name: String,
    /// Full path to executable if resolvable
    pub process_path: String,
    /// Window class name, e.g. "CabinetWClass"
    pub window_class: String,
    /// Window title caption
    pub window_title: String,
}

/// Foreground window handle and boundary snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowSnapshot {
    pub hwnd: isize,
    pub class_name: String,
    pub window_title: String,
    pub rect: [i32; 4], // [left, top, right, bottom]
}

/// Process image metadata and elevation status.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub image_name: String,
    pub image_path: String,
    pub is_elevated: bool,
}

/// Physical monitor bounding box and DPI characteristics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MonitorSnapshot {
    pub hmonitor: isize,
    pub virtual_rect: [i32; 4],
    pub work_area: [i32; 4],
    pub dpi: u32,
}

/// Snapshot of the system clipboard state at invocation timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardSnapshot {
    pub sequence_number: u32,
    pub is_locked: bool,
    pub has_text: bool,
    pub has_files: bool,
    pub text_preview: Option<String>,
}

/// Immutable, point-in-time environmental snapshot captured at the exact
/// microsecond of trigger qualification (INV-CTX-001).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InvocationContext {
    pub invocation_id: u64,
    pub timestamp_qpc: i64,
    pub window: WindowSnapshot,
    pub process: ProcessSnapshot,
    pub cursor_pos: Point,
    pub monitor: MonitorSnapshot,
    pub clipboard: ClipboardSnapshot,
}

impl From<&InvocationContext> for WindowContext {
    fn from(ctx: &InvocationContext) -> Self {
        Self {
            process_name: ctx.process.image_name.clone(),
            process_path: ctx.process.image_path.clone(),
            window_class: ctx.window.class_name.clone(),
            window_title: ctx.window.window_title.clone(),
        }
    }
}

/// Atomically captures the complete immutable `InvocationContext` snapshot.
/// Guarantees execution in $\le 2.0\,\text{ms}$ (INV-CTX-002).
pub fn capture_invocation_context(cursor: Point) -> InvocationContext {
    let invocation_id = INVOCATION_SEQ.fetch_add(1, Ordering::SeqCst);
    let mut qpc: i64 = 0;
    unsafe {
        let _ = QueryPerformanceCounter(&mut qpc);
    }

    unsafe {
        let hwnd = GetForegroundWindow();

        // 1. Window Snapshot
        let mut window = WindowSnapshot::default();
        let mut process = ProcessSnapshot::default();

        if !hwnd.0.is_null() {
            window.hwnd = hwnd.0 as isize;

            let mut win_rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut win_rect);
            window.rect = [win_rect.left, win_rect.top, win_rect.right, win_rect.bottom];

            // Window class
            let mut class_buf = [0u16; 256];
            let class_len = GetClassNameW(hwnd, &mut class_buf);
            if class_len > 0 {
                window.class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
            }

            // Window title
            let mut title_buf = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, &mut title_buf);
            if title_len > 0 {
                window.window_title = String::from_utf16_lossy(&title_buf[..title_len as usize]);
            }

            // 2. Process Snapshot
            let mut pid: u32 = 0;
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
            process.pid = pid;

            if pid != 0 {
                let proc_handle_res = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                    .or_else(|_| OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid));

                if let Ok(proc_handle) = proc_handle_res {
                    // Try QueryFullProcessImageNameW first (standard for PROCESS_QUERY_LIMITED_INFORMATION)
                    let mut img_buf = [0u16; 1024];
                    let mut img_len = img_buf.len() as u32;

                    let query_res = QueryFullProcessImageNameW(
                        proc_handle,
                        PROCESS_NAME_WIN32,
                        windows::core::PWSTR(img_buf.as_mut_ptr()),
                        &mut img_len,
                    );

                    let mut full_path = String::new();
                    if query_res.is_ok() && img_len > 0 {
                        full_path = String::from_utf16_lossy(&img_buf[..img_len as usize]);
                    } else {
                        // Fallback to GetModuleFileNameExW
                        let mut mod_buf = [0u16; 1024];
                        let mod_len = GetModuleFileNameExW(proc_handle, None, &mut mod_buf);
                        if mod_len > 0 {
                            full_path = String::from_utf16_lossy(&mod_buf[..mod_len as usize]);
                        }
                    }

                    if !full_path.is_empty() {
                        let path_obj = Path::new(&full_path);
                        if let Some(name) = path_obj.file_name() {
                            process.image_name = name.to_string_lossy().to_lowercase();
                        }
                        process.image_path = full_path;
                    }

                    // Check process elevation token
                    let mut token = HANDLE::default();
                    if OpenProcessToken(proc_handle, TOKEN_QUERY, &mut token).is_ok() {
                        let mut elevation = TOKEN_ELEVATION::default();
                        let mut ret_len = 0u32;
                        if GetTokenInformation(
                            token,
                            TokenElevation,
                            Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
                            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                            &mut ret_len,
                        )
                        .is_ok()
                        {
                            process.is_elevated = elevation.TokenIsElevated != 0;
                        }
                        let _ = CloseHandle(token);
                    }

                    let _ = CloseHandle(proc_handle);
                }
            }
        }

        // 3. Monitor Snapshot
        let mut monitor = MonitorSnapshot::default();
        let pt = POINT { x: cursor.x, y: cursor.y };
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if !hmon.0.is_null() {
            monitor.hmonitor = hmon.0 as isize;
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(hmon, &mut mi).as_bool() {
                monitor.virtual_rect = [
                    mi.rcMonitor.left,
                    mi.rcMonitor.top,
                    mi.rcMonitor.right,
                    mi.rcMonitor.bottom,
                ];
                monitor.work_area = [
                    mi.rcWork.left,
                    mi.rcWork.top,
                    mi.rcWork.right,
                    mi.rcWork.bottom,
                ];
            }

            let dpi = if !hwnd.0.is_null() {
                GetDpiForWindow(hwnd)
            } else {
                96
            };
            monitor.dpi = if dpi > 0 { dpi } else { 96 };
        }

        // 4. Non-Blocking Clipboard Snapshot (INV-CTX-003)
        let sequence_number = GetClipboardSequenceNumber();
        let has_text = IsClipboardFormatAvailable(CF_UNICODETEXT_VAL).is_ok()
            || IsClipboardFormatAvailable(CF_TEXT_VAL).is_ok();
        let has_files = IsClipboardFormatAvailable(CF_HDROP_VAL).is_ok();

        // Non-blocking clipboard lock test (never hangs invocation)
        let is_locked = if OpenClipboard(HWND::default()).is_ok() {
            let _ = CloseClipboard();
            false
        } else {
            true
        };

        let clipboard = ClipboardSnapshot {
            sequence_number,
            is_locked,
            has_text,
            has_files,
            text_preview: None,
        };

        InvocationContext {
            invocation_id,
            timestamp_qpc: qpc,
            window,
            process,
            cursor_pos: cursor,
            monitor,
            clipboard,
        }
    }
}

/// Retrieves the current foreground window context with sub-millisecond overhead.
/// (Maintained for lightweight profile matching and backward compatibility).
pub fn active_window_context() -> WindowContext {
    let mut pt = POINT::default();
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
    }
    let inv = capture_invocation_context(Point::new(pt.x, pt.y));
    WindowContext::from(&inv)
}

/// Checks whether a specific process executable (e.g. "code.exe" or "notepad.exe") is currently running anywhere on the system.
pub fn is_process_running(target_process_name: &str) -> bool {
    let target_lower = target_process_name.trim().to_lowercase();
    unsafe {
        let snapshot: HANDLE = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe_name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();

                if exe_name == target_lower {
                    let _ = CloseHandle(snapshot);
                    return true;
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }
    false
}
