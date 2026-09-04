use std::io;

/// Executes an arbitrary system command or binary asynchronously and detached.
/// On Windows, runs via `cmd.exe /C` with `CREATE_NO_WINDOW` (0x08000000)
/// so that launching GUI apps (e.g. sublime.exe) does not flash a terminal window,
/// while allowing standard Windows PATH and argument resolution.
pub fn execute_command(cmd_str: &str) -> io::Result<()> {
    if cmd_str.trim().is_empty() {
        return Ok(());
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        std::process::Command::new("cmd.exe")
            .raw_arg(format!("/C {}", cmd_str))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    }

    #[cfg(not(windows))]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd_str)
            .spawn()?;
    }

    Ok(())
}
