use std::os::windows::process::CommandExt;
use winpie::app::Application;

const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn print_help() {
    println!("WinPie - High-Performance Radial Menu for Windows\n");
    println!("USAGE:");
    println!("  winpie                       Start WinPie radial menu daemon in the background");
    println!("  winpie kill                  Stop the running WinPie process");
    println!("  winpie reload                Restart / reload WinPie with updated config");
    println!("  winpie status                Check if WinPie is currently running");
    println!("  winpie inspect               Live stream foreground window context & profile matching");
    println!("  winpie autostart enable      Launch WinPie automatically on Windows login");
    println!("  winpie autostart disable     Remove WinPie from Windows startup");
    println!("  winpie autostart status      Check Windows startup shortcut status");
    println!("  winpie --foreground          Run in foreground attached to terminal (debug mode)");
    println!("  winpie help                  Show this help message\n");
    println!("DEFAULT CONTROLS:");
    println!("  Win + Esc                    Activate radial pie menu");
    println!("  Left Click                   Commit sector or enter submenu");
    println!("  Release Win Key              Commit hovered sector");
    println!("  Right Click / Esc            Cancel menu\n");
}

fn run_inspector() -> Result<(), Box<dyn std::error::Error>> {
    let config = winpie::config::AppConfig::load_or_default("config/default.yaml");
    println!("============================================================");
    println!("  WinPie Window Context & Profile Inspector");
    println!("============================================================");
    println!("Active Profiles Configured: {}", config.profiles.len());
    for (i, p) in config.profiles.iter().enumerate() {
        println!("  [{}] {} (match: process={:?}, class={:?}, title={:?})", 
            i, p.name, p.match_rules.process, p.match_rules.window_class, p.match_rules.window_title);
    }
    println!("\nStreaming foreground window changes... Press Ctrl+C to exit.\n");

    let mut last_ctx = winpie::context::WindowContext::default();

    loop {
        let ctx = winpie::context::active_window_context();
        if ctx != last_ctx && (!ctx.process_name.is_empty() || !ctx.window_title.is_empty()) {
            println!("------------------------------------------------------------");
            println!("[FOCUS CHANGED]");
            println!("  Executable : {}", if ctx.process_name.is_empty() { "<unknown>" } else { &ctx.process_name });
            if !ctx.process_path.is_empty() {
                println!("  Path       : {}", ctx.process_path);
            }
            if !ctx.window_class.is_empty() {
                println!("  Class      : {}", ctx.window_class);
            }
            if !ctx.window_title.is_empty() {
                println!("  Title      : {}", ctx.window_title);
            }

            let profile_idx = config.find_matching_profile(&ctx);
            match profile_idx {
                Some(idx) => {
                    let profile = &config.profiles[idx];
                    println!("  Matched    : Profile #{} -> \"{}\"", idx, profile.name);
                    println!("  Sector Bindings:");
                    for sector in &[
                        winpie::geometry::Sector::N,
                        winpie::geometry::Sector::NE,
                        winpie::geometry::Sector::E,
                        winpie::geometry::Sector::SE,
                        winpie::geometry::Sector::S,
                        winpie::geometry::Sector::SW,
                        winpie::geometry::Sector::W,
                        winpie::geometry::Sector::NW,
                    ] {
                        let label = config.get_label_for_sector_with_profile(*sector, Some(idx));
                        let is_override = profile.wheel.labels.contains_key(sector.name()) || profile.wheel.menus.contains_key(sector.name());
                        println!("    [{:>2}] {:<18} {}", sector.name(), label, if is_override { "(custom profile)" } else { "(inherited global)" });
                    }
                }
                None => {
                    println!("  Matched    : [Global Default] (no specific profile matched)");
                }
            }
            println!();
            last_ctx = ctx;
        }

        std::thread::sleep(std::time::Duration::from_millis(150));
    }
}

fn spawn_detached_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    let _ = std::process::Command::new(current_exe)
        .arg("--daemon")
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "inspect" | "context" | "test-context" => {
                return run_inspector();
            }
            "kill" | "stop" => {
                if winpie::ipc::signal_kill() {
                    println!("[WinPie] Successfully stopped running WinPie process.");
                } else {
                    println!("[WinPie] No active WinPie process found.");
                }
                return Ok(());
            }
            "reload" | "restart" => {
                println!("[WinPie] Reloading WinPie...");
                if winpie::ipc::is_running() {
                    let _ = winpie::ipc::signal_kill();
                }
                spawn_detached_daemon()?;
                println!("[WinPie] WinPie restarted in background (Press Win+Esc to activate).");
                return Ok(());
            }
            "status" => {
                if winpie::ipc::is_running() {
                    println!("[WinPie] Status: Running in background.");
                } else {
                    println!("[WinPie] Status: Stopped (not running).");
                }
                return Ok(());
            }
            "autostart" => {
                let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
                match sub {
                    "enable" | "on" => {
                        winpie::ipc::set_autostart(true)?;
                        println!("[WinPie] Autostart enabled (Shortcut added to Windows Startup folder).");
                    }
                    "disable" | "off" => {
                        winpie::ipc::set_autostart(false)?;
                        println!("[WinPie] Autostart disabled (Shortcut removed from Windows Startup folder).");
                    }
                    _ => {
                        let enabled = winpie::ipc::autostart_status();
                        println!(
                            "[WinPie] Autostart status: {}",
                            if enabled {
                                "Enabled (Spawns on Windows login)"
                            } else {
                                "Disabled"
                            }
                        );
                        println!("Usage: winpie autostart [enable|disable|status]");
                    }
                }
                return Ok(());
            }
            "--daemon" => {
                // Background daemon execution loop (spawned detached)
                let _mutex_handle = match winpie::ipc::try_acquire_singleton() {
                    Some(handle) => handle,
                    None => return Ok(()), // Already running
                };
                let mut app = Application::new()?;
                return app.run(true);
            }
            "--foreground" | "-f" => {
                // Foreground attached execution loop (for dev runner / debugging)
                let _mutex_handle = match winpie::ipc::try_acquire_singleton() {
                    Some(handle) => handle,
                    None => {
                        println!("[WinPie] WinPie is already running in the background.");
                        println!("Run 'winpie help' for available CLI commands or 'winpie reload' to restart.\n");
                        return Ok(());
                    }
                };
                let mut app = Application::new()?;
                return app.run(false);
            }
            "help" | "--help" | "-h" | "/?" => {
                print_help();
                return Ok(());
            }
            unknown => {
                println!("[WinPie] Unknown command: '{}'\n", unknown);
                print_help();
                return Ok(());
            }
        }
    }

    // Default `winpie` execution: check if already running; if not, spawn background daemon and return prompt
    if winpie::ipc::is_running() {
        println!("[WinPie] WinPie is already running in the background.");
        println!("Run 'winpie help' for available CLI commands or 'winpie reload' to restart.\n");
        return Ok(());
    }

    spawn_detached_daemon()?;
    println!("[WinPie] WinPie daemon started in the background.");
    println!("- Press Win+Esc anywhere to open the radial menu.");
    println!("- Run 'winpie help' for CLI commands or 'winpie kill' to stop.");

    Ok(())
}
