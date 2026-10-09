use std::io;
use crate::context::InvocationContext;

/// Executes an arbitrary system command or script asynchronously and detached,
/// injecting contextual environment variables from the frozen `InvocationContext` snapshot (Phase 10).
pub fn execute_command_with_context(
    cmd_str: &str,
    context: Option<&InvocationContext>,
) -> io::Result<()> {
    let clean = cmd_str.trim();
    if clean.is_empty() {
        return Ok(());
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = if clean.starts_with("ps:") {
            let script = clean.trim_start_matches("ps:").trim();
            let mut c = std::process::Command::new("powershell.exe");
            c.args(["-NoProfile", "-NonInteractive", "-Command", script]);
            c
        } else if clean.ends_with(".ps1") || clean.contains(".ps1 ") {
            let mut c = std::process::Command::new("powershell.exe");
            c.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]);
            c.raw_arg(clean);
            c
        } else {
            let mut c = std::process::Command::new("cmd.exe");
            c.raw_arg(format!("/C {}", clean));
            c
        };

        cmd.creation_flags(CREATE_NO_WINDOW);

        // Inject contextual environment variables from frozen snapshot
        if let Some(ctx) = context {
            cmd.env("WINPIE_CONTEXT_HWND", ctx.window.hwnd.to_string());
            cmd.env("WINPIE_CONTEXT_PROCESS_NAME", &ctx.process.image_name);
            cmd.env("WINPIE_CONTEXT_PROCESS_PATH", &ctx.process.image_path);
            cmd.env("WINPIE_CONTEXT_WINDOW_CLASS", &ctx.window.class_name);
            cmd.env("WINPIE_CONTEXT_WINDOW_TITLE", &ctx.window.window_title);
            cmd.env("WINPIE_CONTEXT_CURSOR_X", ctx.cursor_pos.x.to_string());
            cmd.env("WINPIE_CONTEXT_CURSOR_Y", ctx.cursor_pos.y.to_string());
            cmd.env("WINPIE_CONTEXT_DPI", ctx.monitor.dpi.to_string());
            cmd.env(
                "WINPIE_CONTEXT_IS_ELEVATED",
                if ctx.process.is_elevated { "1" } else { "0" },
            );
            cmd.env(
                "WINPIE_CONTEXT_CLIPBOARD_HAS_TEXT",
                if ctx.clipboard.has_text { "1" } else { "0" },
            );
            cmd.env(
                "WINPIE_CONTEXT_CLIPBOARD_HAS_FILES",
                if ctx.clipboard.has_files { "1" } else { "0" },
            );
        }

        cmd.spawn()?;
    }

    #[cfg(not(windows))]
    {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(clean);
        if let Some(ctx) = context {
            cmd.env("WINPIE_CONTEXT_PROCESS_NAME", &ctx.process.image_name);
            cmd.env("WINPIE_CONTEXT_WINDOW_TITLE", &ctx.window.window_title);
        }
        cmd.spawn()?;
    }

    Ok(())
}

/// Convenience wrapper for backward compatibility.
pub fn execute_command(cmd_str: &str) -> io::Result<()> {
    execute_command_with_context(cmd_str, None)
}
