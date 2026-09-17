use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const CONTROL_WINDOW_CLASS: windows::core::PCWSTR = windows::core::w!("WinPieControlWindowClass");
pub const CONTROL_WINDOW_TITLE: windows::core::PCWSTR = windows::core::w!("WinPie Control Window");
pub const SINGLETON_MUTEX_NAME: windows::core::PCWSTR = windows::core::w!("Local\\WinPie_SingleInstance_Mutex");

pub const WM_WINPIE_CONTROL_KILL: u32 = WM_USER + 101;

/// Hidden message-only window used for inter-process communication and remote control.
pub struct ControlWindow {
    hwnd: HWND,
}

impl ControlWindow {
    pub fn new(instance: HINSTANCE) -> Result<Self, windows::core::Error> {
        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(control_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: HICON::default(),
                hCursor: HCURSOR::default(),
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
                lpszMenuName: windows::core::PCWSTR::null(),
                lpszClassName: CONTROL_WINDOW_CLASS,
                hIconSm: HICON::default(),
            };

            RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CONTROL_WINDOW_CLASS,
                CONTROL_WINDOW_TITLE,
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                instance,
                None,
            )?;

            Ok(Self { hwnd })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for ControlWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.0.is_null() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe extern "system" fn control_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE | WM_WINPIE_CONTROL_KILL => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Checks if an instance of WinPie is already active.
pub fn is_running() -> bool {
    unsafe {
        let hwnd = FindWindowW(CONTROL_WINDOW_CLASS, CONTROL_WINDOW_TITLE);
        if let Ok(h) = hwnd {
            if !h.0.is_null() {
                return true;
            }
        }
    }
    // Check if the singleton mutex is held by another process
    try_acquire_singleton().is_none()
}

/// Acquires the singleton mutex. Returns `Some(HANDLE)` on success, or `None` if an instance already exists.
pub fn try_acquire_singleton() -> Option<HANDLE> {
    unsafe {
        let handle = CreateMutexW(None, true, SINGLETON_MUTEX_NAME).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return None;
        }
        Some(handle)
    }
}

/// Sends a kill/quit signal to any running WinPie instance.
/// Waits up to 2 seconds for clean exit.
pub fn signal_kill() -> bool {
    let mut found = false;
    unsafe {
        let hwnd = FindWindowW(CONTROL_WINDOW_CLASS, CONTROL_WINDOW_TITLE);
        if let Ok(h) = hwnd {
            if !h.0.is_null() {
                let _ = PostMessageW(h, WM_CLOSE, WPARAM(0), LPARAM(0));
                found = true;
            }
        }
    }

    if found {
        // Wait up to 1 second for graceful exit
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(100));
            if !is_running() {
                return true;
            }
        }
    }

    // Fallback if needed: terminate other winpie processes excluding current PID
    let current_pid = std::process::id();
    let filter = format!("PID ne {}", current_pid);
    let status = Command::new("taskkill")
        .args(["/F", "/FI", &filter, "/IM", "winpie.exe"])
        .output();

    if let Ok(out) = status {
        if out.status.success() {
            return true;
        }
    }

    found
}

/// Resolves the Windows Startup directory shortcut path.
pub fn get_startup_shortcut_path() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let mut path = PathBuf::from(appdata);
        path.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
        path.push("WinPie.lnk");
        Some(path)
    } else {
        None
    }
}

/// Checks if the WinPie Windows Startup shortcut is enabled.
pub fn autostart_status() -> bool {
    if let Some(path) = get_startup_shortcut_path() {
        path.exists()
    } else {
        false
    }
}

/// Enables or disables WinPie launching on Windows startup.
pub fn set_autostart(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut_path = get_startup_shortcut_path()
        .ok_or("Failed to resolve Windows APPDATA environment variable")?;

    if enable {
        let current_exe = std::env::current_exe()?;
        let target_str = current_exe.to_string_lossy();
        let parent_dir = current_exe.parent().unwrap_or(&current_exe).to_string_lossy();
        let link_str = shortcut_path.to_string_lossy();

        // Use PowerShell WScript.Shell COM object to create rock-solid Windows shortcut
        let ps_script = format!(
            "$ws = New-Object -ComObject WScript.Shell; $s = $ws.CreateShortcut('{}'); $s.TargetPath = '{}'; $s.WorkingDirectory = '{}'; $s.Description = 'WinPie Radial Menu Daemon'; $s.Save()",
            link_str, target_str, parent_dir
        );

        let status = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .status()?;

        if !status.success() {
            return Err("PowerShell failed to create startup shortcut".into());
        }
    } else if shortcut_path.exists() {
        std::fs::remove_file(&shortcut_path)?;
    }

    Ok(())
}
