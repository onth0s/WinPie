use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

/// Snapshot of the active foreground window context.
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

/// Retrieves the current foreground window context with sub-millisecond overhead.
pub fn active_window_context() -> WindowContext {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return WindowContext::default();
        }

        let mut pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let mut process_name = String::new();
        let mut process_path = String::new();

        if pid != 0 {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                .or_else(|_| OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid));

            if let Ok(proc_handle) = handle {
                let mut buf = [0u16; 1024];
                let len = GetModuleFileNameExW(proc_handle, None, &mut buf);
                let _ = CloseHandle(proc_handle);

                if len > 0 {
                    let full_path = String::from_utf16_lossy(&buf[..len as usize]);
                    let path_obj = Path::new(&full_path);
                    if let Some(name) = path_obj.file_name() {
                        process_name = name.to_string_lossy().to_lowercase();
                    }
                    process_path = full_path;
                }
            }
        }

        // Window class name
        let mut class_buf = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class_buf);
        let window_class = if class_len > 0 {
            String::from_utf16_lossy(&class_buf[..class_len as usize])
        } else {
            String::new()
        };

        // Window title
        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let window_title = if title_len > 0 {
            String::from_utf16_lossy(&title_buf[..title_len as usize])
        } else {
            String::new()
        };

        WindowContext {
            process_name,
            process_path,
            window_class,
            window_title,
        }
    }
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
