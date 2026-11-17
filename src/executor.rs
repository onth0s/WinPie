use std::collections::BTreeMap;
use std::io;
use std::process::Stdio;
use crate::context::InvocationContext;

/// Parses optional `admin:` prefix from a command string, returning `(wants_admin, inner_command)`.
pub fn parse_command_prefix(cmd_str: &str) -> (bool, &str) {
    let trimmed = cmd_str.trim();
    if let Some(rest) = trimmed.strip_prefix("admin:") {
        (true, rest.trim())
    } else {
        (false, trimmed)
    }
}

/// Resolves a command string (after `admin:` prefix stripping) into `(executable, raw_arguments)`.
#[cfg(windows)]
fn resolve_windows_command(clean: &str) -> (&'static str, String) {
    if clean.starts_with("ps:") {
        let script = clean.trim_start_matches("ps:").trim();
        let escaped = script.replace('"', "\\\"");
        (
            "powershell.exe",
            format!("-NoProfile -NonInteractive -Command \"{}\"", escaped),
        )
    } else if clean.ends_with(".ps1") || clean.contains(".ps1 ") {
        (
            "powershell.exe",
            format!("-NoProfile -NonInteractive -ExecutionPolicy Bypass -File {}", clean),
        )
    } else {
        let formatted_cmd = if clean.contains(' ') && !clean.starts_with('"') {
            let path_buf = std::path::PathBuf::from(clean);
            if path_buf.is_file() {
                format!("/C \"\"{}\"\"", clean)
            } else if let Some(first_space) = clean.find(' ') {
                let first_part = &clean[..first_space];
                let rest = &clean[first_space..];
                if std::path::Path::new(first_part).is_file() {
                    format!("/C \"\"{}\"{}\"", first_part, rest)
                } else {
                    format!("/C {}", clean)
                }
            } else {
                format!("/C {}", clean)
            }
        } else {
            format!("/C {}", clean)
        };
        ("cmd.exe", formatted_cmd)
    }
}

/// Builds the list of `WINPIE_CONTEXT_*` environment variables from an `InvocationContext`.
fn build_context_env_vars(context: Option<&InvocationContext>) -> Vec<(String, String)> {
    let Some(ctx) = context else {
        return Vec::new();
    };
    vec![
        ("WINPIE_CONTEXT_INVOCATION_ID".into(), ctx.invocation_id.to_string()),
        ("WINPIE_CONTEXT_PID".into(), ctx.process.pid.to_string()),
        ("WINPIE_CONTEXT_HWND".into(), ctx.window.hwnd.to_string()),
        ("WINPIE_CONTEXT_EXE".into(), ctx.process.image_name.clone()),
        ("WINPIE_CONTEXT_PROCESS_NAME".into(), ctx.process.image_name.clone()),
        ("WINPIE_CONTEXT_PROCESS_PATH".into(), ctx.process.image_path.clone()),
        ("WINPIE_CONTEXT_WINDOW_CLASS".into(), ctx.window.class_name.clone()),
        ("WINPIE_CONTEXT_WINDOW_TITLE".into(), ctx.window.window_title.clone()),
        ("WINPIE_CONTEXT_CURSOR_X".into(), ctx.cursor_pos.x.to_string()),
        ("WINPIE_CONTEXT_CURSOR_Y".into(), ctx.cursor_pos.y.to_string()),
        ("WINPIE_CONTEXT_DPI".into(), ctx.monitor.dpi.to_string()),
        (
            "WINPIE_CONTEXT_IS_ELEVATED".into(),
            if ctx.process.is_elevated { "1" } else { "0" }.into(),
        ),
        (
            "WINPIE_CONTEXT_CLIPBOARD_HAS_TEXT".into(),
            if ctx.clipboard.has_text { "1" } else { "0" }.into(),
        ),
        (
            "WINPIE_CONTEXT_CLIPBOARD_HAS_FILES".into(),
            if ctx.clipboard.has_files { "1" } else { "0" }.into(),
        ),
    ]
}

/// Builds a Windows `CREATE_UNICODE_ENVIRONMENT` block (`KEY=VALUE\0...\0\0`),
/// sorted case-insensitively by key as required by `CreateProcessWithTokenW`.
pub fn build_unicode_env_block(extra: &[(String, String)]) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let mut merged: BTreeMap<String, (String, String)> = BTreeMap::new();
    for (k, v) in std::env::vars() {
        merged.insert(k.to_uppercase(), (k, v));
    }
    for (k, v) in extra {
        merged.insert(k.to_uppercase(), (k.clone(), v.clone()));
    }

    let mut block = Vec::new();
    for (_, (k, v)) in merged {
        let entry = format!("{}={}", k, v);
        block.extend(OsStr::new(&entry).encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}

#[cfg(windows)]
fn spawn_standard(
    clean: &str,
    program: &str,
    raw_args: &str,
    env_vars: &[(String, String)],
) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = if clean.starts_with("ps:") {
        let script = clean.trim_start_matches("ps:").trim();
        let mut c = std::process::Command::new("powershell.exe");
        c.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        c
    } else {
        let mut c = std::process::Command::new(program);
        c.raw_arg(raw_args);
        c
    };

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);

    for (k, v) in env_vars {
        cmd.env(k, v);
    }

    cmd.spawn()?;
    Ok(())
}

/// Spawns a command with the desktop shell's (`explorer.exe`) non-elevated Medium Integrity token
/// via `CreateProcessWithTokenW`, preventing Admin token inheritance while preserving
/// `WINPIE_CONTEXT_*` environment variables and `CREATE_NO_WINDOW`.
#[cfg(windows)]
fn spawn_de_elevated(
    program: &str,
    raw_args: &str,
    env_vars: &[(String, String)],
) -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        DuplicateTokenEx, SecurityImpersonation, TokenPrimary, TOKEN_ALL_ACCESS, TOKEN_DUPLICATE,
        TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{
        CreateProcessWithTokenW, OpenProcess, OpenProcessToken, CREATE_NO_WINDOW,
        CREATE_UNICODE_ENVIRONMENT, LOGON_WITH_PROFILE, PROCESS_INFORMATION,
        PROCESS_QUERY_LIMITED_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId, SW_HIDE};

    unsafe {
        let shell_hwnd = GetShellWindow();
        if shell_hwnd.0.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "GetShellWindow returned null",
            ));
        }

        let mut shell_pid: u32 = 0;
        GetWindowThreadProcessId(shell_hwnd, Some(&mut shell_pid));
        if shell_pid == 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Failed to resolve shell PID",
            ));
        }

        let proc_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, shell_pid)
            .map_err(io::Error::other)?;

        let mut shell_token = HANDLE::default();
        if let Err(e) = OpenProcessToken(proc_handle, TOKEN_DUPLICATE | TOKEN_QUERY, &mut shell_token) {
            let _ = CloseHandle(proc_handle);
            return Err(io::Error::other(e));
        }

        let mut primary_token = HANDLE::default();
        let dup_res = DuplicateTokenEx(
            shell_token,
            TOKEN_ALL_ACCESS,
            None,
            SecurityImpersonation,
            TokenPrimary,
            &mut primary_token,
        );
        let _ = CloseHandle(shell_token);
        let _ = CloseHandle(proc_handle);
        dup_res.map_err(io::Error::other)?;

        let full_cmd = format!("{} {}", program, raw_args);
        let mut cmd_wide: Vec<u16> = OsStr::new(&full_cmd)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let env_block = build_unicode_env_block(env_vars);

        let si = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            dwFlags: STARTF_USESHOWWINDOW,
            wShowWindow: SW_HIDE.0 as u16,
            ..Default::default()
        };
        let mut pi = PROCESS_INFORMATION::default();

        let create_res = CreateProcessWithTokenW(
            primary_token,
            LOGON_WITH_PROFILE,
            None,
            PWSTR(cmd_wide.as_mut_ptr()),
            CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT,
            Some(env_block.as_ptr() as *const std::ffi::c_void),
            None,
            &si,
            &mut pi,
        );

        let _ = CloseHandle(primary_token);

        match create_res {
            Ok(()) => {
                if !pi.hProcess.is_invalid() {
                    let _ = CloseHandle(pi.hProcess);
                }
                if !pi.hThread.is_invalid() {
                    let _ = CloseHandle(pi.hThread);
                }
                Ok(())
            }
            Err(e) => Err(io::Error::other(e)),
        }
    }
}

/// Launches an explicit `admin:` command via Windows UAC (`ShellExecuteW` `"runas"` verb)
/// when WinPie itself is not elevated.
#[cfg(windows)]
fn spawn_elevated_uac(program: &str, raw_args: &str) -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let verb: Vec<u16> = OsStr::new("runas").encode_wide().chain(Some(0)).collect();
    let prog_wide: Vec<u16> = OsStr::new(program).encode_wide().chain(Some(0)).collect();
    let args_wide: Vec<u16> = OsStr::new(raw_args).encode_wide().chain(Some(0)).collect();

    unsafe {
        let res = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(prog_wide.as_ptr()),
            PCWSTR(args_wide.as_ptr()),
            None,
            SW_HIDE,
        );
        if res.0 as usize <= 32 {
            return Err(io::Error::other(format!(
                "ShellExecuteW runas failed with code {}",
                res.0 as usize
            )));
        }
    }
    Ok(())
}

/// Executes an arbitrary system command or script asynchronously and detached,
/// injecting contextual environment variables from the frozen `InvocationContext` snapshot.
///
/// By default, even when WinPie runs elevated (`daemon.run_as_admin: true`), commands are
/// spawned de-elevated with the desktop shell's Medium Integrity token so child apps do not
/// inherit Administrator privileges. Prefixing a command with `admin:` (or setting
/// `run_as_admin: true` on a menu item) explicitly opts into elevation for that command.
pub fn execute_command_with_context(
    cmd_str: &str,
    context: Option<&InvocationContext>,
) -> io::Result<()> {
    let (wants_admin, clean) = parse_command_prefix(cmd_str);
    if clean.is_empty() {
        return Ok(());
    }

    let env_vars = build_context_env_vars(context);

    #[cfg(windows)]
    {
        let (program, raw_args) = resolve_windows_command(clean);
        let is_elevated = crate::context::is_current_process_elevated();

        if wants_admin {
            if is_elevated {
                spawn_standard(clean, program, &raw_args, &env_vars)
            } else {
                spawn_elevated_uac(program, &raw_args)
            }
        } else if is_elevated {
            match spawn_de_elevated(program, &raw_args, &env_vars) {
                Ok(()) => Ok(()),
                Err(e) => {
                    crate::diagnostics::log_raw(&format!(
                        "[WinPie:EXECUTOR] De-elevated spawn failed ({}); falling back to standard spawn.",
                        e
                    ));
                    spawn_standard(clean, program, &raw_args, &env_vars)
                }
            }
        } else {
            spawn_standard(clean, program, &raw_args, &env_vars)
        }
    }

    #[cfg(not(windows))]
    {
        let _ = wants_admin;
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(clean);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for (k, v) in env_vars {
            cmd.env(k, v);
        }
        cmd.spawn()?;
        Ok(())
    }
}

/// Convenience wrapper for backward compatibility.
pub fn execute_command(cmd_str: &str) -> io::Result<()> {
    execute_command_with_context(cmd_str, None)
}

#[cfg(test)]
mod tests {
    use super::{build_unicode_env_block, parse_command_prefix};

    #[test]
    fn test_parse_command_prefix_admin_and_plain() {
        assert_eq!(parse_command_prefix("dimly"), (false, "dimly"));
        assert_eq!(parse_command_prefix("  admin: taskmgr.exe "), (true, "taskmgr.exe"));
        assert_eq!(parse_command_prefix("admin: ps: Get-Process"), (true, "ps: Get-Process"));
        assert_eq!(parse_command_prefix("admin:   "), (true, ""));
    }

    #[test]
    fn test_build_unicode_env_block_terminators_and_override() {
        let extra = vec![("WINPIE_TEST_KEY".to_string(), "value123".to_string())];
        let block = build_unicode_env_block(&extra);
        assert!(block.len() >= 2);
        // Must end with double null terminator
        assert_eq!(block[block.len() - 1], 0);
        assert_eq!(block[block.len() - 2], 0);

        let decoded = String::from_utf16_lossy(&block);
        assert!(decoded.contains("WINPIE_TEST_KEY=value123\0"));
    }
}
