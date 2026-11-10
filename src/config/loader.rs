use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use super::schema::AppConfig;

static OVERRIDE_CONFIG_PATH: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Sets an explicit configuration path override (e.g. from `--config <path>` or parent process).
pub fn set_override_config_path(path: PathBuf) {
    if let Ok(mut lock) = OVERRIDE_CONFIG_PATH.write() {
        *lock = Some(path);
    }
}

/// Returns the explicitly set configuration path override, if any.
pub fn get_override_config_path() -> Option<PathBuf> {
    OVERRIDE_CONFIG_PATH.read().ok().and_then(|lock| lock.clone())
}

/// Discovers the path to the active configuration file in order of priority:
/// 0. Explicit override path (set via `--config` CLI arg)
/// 1. `WINPIE_CONFIG` environment variable
/// 2. `config.yaml` in current working directory (CWD)
/// 3. `config.yaml` located in `%APPDATA%\WinPie\config.yaml`
/// 4. `config.yaml` located in the directory of the running executable
/// 5. Developer workspace directory fallback (`env!("CARGO_MANIFEST_DIR")/config.yaml`)
/// 6. `config.example.yaml` fallbacks (CWD -> %APPDATA% -> Exe Dir -> Workspace Dir)
pub fn discover_config_path() -> Option<PathBuf> {
    // 0. Explicit CLI override
    if let Some(override_path) = get_override_config_path() {
        if override_path.is_file() {
            if let Ok(abs) = fs::canonicalize(&override_path) {
                return Some(abs);
            }
            return Some(override_path);
        }
    }

    // 1. Environment variable
    if let Ok(env_path) = std::env::var("WINPIE_CONFIG") {
        let p = PathBuf::from(env_path);
        if p.is_file() {
            if let Ok(abs) = fs::canonicalize(&p) {
                return Some(abs);
            }
            return Some(p);
        }
    }

    // 2. CWD config.yaml
    let cwd_config = Path::new("config.yaml");
    if cwd_config.is_file() {
        if let Ok(abs) = fs::canonicalize(cwd_config) {
            return Some(abs);
        }
        return Some(cwd_config.to_path_buf());
    }

    // 3. %APPDATA%\WinPie\config.yaml
    if let Ok(appdata) = std::env::var("APPDATA") {
        let appdata_config = PathBuf::from(appdata).join("WinPie").join("config.yaml");
        if appdata_config.is_file() {
            if let Ok(abs) = fs::canonicalize(&appdata_config) {
                return Some(abs);
            }
            return Some(appdata_config);
        }
    }

    // 4. Next to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let exe_dir_config = parent.join("config.yaml");
            if exe_dir_config.is_file() {
                if let Ok(abs) = fs::canonicalize(&exe_dir_config) {
                    return Some(abs);
                }
                return Some(exe_dir_config);
            }
        }
    }

    // 5. Developer workspace fallback: always locate repo config.yaml even when launched from System32 or C:\
    let manifest_config = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.yaml");
    if manifest_config.is_file() {
        if let Ok(abs) = fs::canonicalize(&manifest_config) {
            return Some(abs);
        }
        return Some(manifest_config);
    }

    // 6. Fallback templates: config.example.yaml
    let cwd_example = Path::new("config.example.yaml");
    if cwd_example.is_file() {
        if let Ok(abs) = fs::canonicalize(cwd_example) {
            return Some(abs);
        }
        return Some(cwd_example.to_path_buf());
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        let appdata_example = PathBuf::from(appdata).join("WinPie").join("config.example.yaml");
        if appdata_example.is_file() {
            if let Ok(abs) = fs::canonicalize(&appdata_example) {
                return Some(abs);
            }
            return Some(appdata_example);
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let exe_dir_example = parent.join("config.example.yaml");
            if exe_dir_example.is_file() {
                if let Ok(abs) = fs::canonicalize(&exe_dir_example) {
                    return Some(abs);
                }
                return Some(exe_dir_example);
            }
        }
    }

    let manifest_example = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.yaml");
    if manifest_example.is_file() {
        if let Ok(abs) = fs::canonicalize(&manifest_example) {
            return Some(abs);
        }
        return Some(manifest_example);
    }

    None
}

/// Automatically sanitizes common YAML string escaping issues, particularly Windows paths
/// with unescaped backslashes inside double quotes (e.g. "C:\Users\..." triggering \U escape errors).
/// Note: Single-quoted YAML strings ('...') already treat backslashes literally and are never modified.
pub fn sanitize_yaml_string(input: &str) -> (String, Vec<usize>) {
    let mut fixed_lines = Vec::new();
    let mut output_lines = Vec::new();

    for (line_idx, line) in input.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim_start();

        // If the line value is enclosed in single quotes ('...'), YAML treats backslashes literally.
        // Do not touch single-quoted string values.
        if let Some(colon_pos) = trimmed.find(':') {
            let after_colon = trimmed[colon_pos + 1..].trim();
            if after_colon.starts_with('\'') && after_colon.ends_with('\'') {
                output_lines.push(line.to_string());
                continue;
            }
        }

        // Check if line contains double-quoted string with Windows path pattern
        if let Some(first_quote) = line.find('"') {
            if let Some(last_quote) = line.rfind('"') {
                if last_quote > first_quote {
                    let before = &line[..first_quote + 1];
                    let inside = &line[first_quote + 1..last_quote];
                    let after = &line[last_quote..];

                    if (inside.contains(":\\") || inside.contains('\\')) && !inside.contains("\\\\") {
                        let mut sanitized_inside = String::with_capacity(inside.len() + 10);
                        let chars: Vec<char> = inside.chars().collect();
                        let mut i = 0;
                        let mut modified = false;

                        while i < chars.len() {
                            if chars[i] == '\\' {
                                if i + 1 < chars.len() {
                                    let next = chars[i + 1];
                                    if next == '\\' || next == '"' {
                                        sanitized_inside.push('\\');
                                        sanitized_inside.push(next);
                                        i += 2;
                                        continue;
                                    }
                                }
                                sanitized_inside.push('\\');
                                sanitized_inside.push('\\');
                                modified = true;
                                i += 1;
                            } else {
                                sanitized_inside.push(chars[i]);
                                i += 1;
                            }
                        }

                        if modified {
                            fixed_lines.push(line_num);
                            output_lines.push(format!("{}{}{}", before, sanitized_inside, after));
                            continue;
                        }
                    }
                }
            }
        }
        output_lines.push(line.to_string());
    }

    let mut result = output_lines.join("\n");
    if input.ends_with('\n') {
        result.push('\n');
    }
    (result, fixed_lines)
}

/// Parses YAML string into AppConfig with automatic unescaped Windows path repair.
/// Returns Ok((AppConfig, auto_fixed_lines)) or Err(detailed_error_message).
pub fn parse_app_config(content: &str) -> Result<(AppConfig, Vec<usize>), String> {
    // 1. First attempt direct parse
    match serde_yaml::from_str::<AppConfig>(content) {
        Ok(cfg) => Ok((cfg, Vec::new())),
        Err(orig_err) => {
            // 2. Attempt auto-repair on failure
            let (sanitized, fixed_lines) = sanitize_yaml_string(content);
            if !fixed_lines.is_empty() {
                if let Ok(cfg) = serde_yaml::from_str::<AppConfig>(&sanitized) {
                    return Ok((cfg, fixed_lines));
                }
            }

            // 3. If still failing, format a precise diagnostic with line, column, and line text
            let err_str = orig_err.to_string();
            let mut line_num = None;
            let mut col_num = None;

            if let Some(loc) = orig_err.location() {
                line_num = Some(loc.line());
                col_num = Some(loc.column());
            }

            let mut msg = format!("YAML error: {}", err_str);
            if let Some(l) = line_num {
                let line_text = content.lines().nth(l.saturating_sub(1)).unwrap_or("");
                msg = format!(
                    "YAML error at line {}, col {}: \"{}\" ({})",
                    l,
                    col_num.unwrap_or(0),
                    line_text.trim(),
                    err_str
                );
            }

            // Provide an actionable hint for the common mistake of placing a submenu
            // mapping under `wheel.commands` (which only accepts command strings).
            if err_str.contains("invalid type: map, expected a string") && err_str.contains("commands") {
                msg.push_str(
                    "\nHint: wheel.commands entries must be command strings (for example \"control.exe\"). \
                     If this block defines a submenu with title/tooltip/items, move it under wheel.menus.<sector> instead. \
                     See config.example.yaml for the canonical layout.",
                );
            }

            Err(msg)
        }
    }
}

impl AppConfig {
    /// Discovers config file path and loads it. Returns (AppConfig, Option<PathBuf>).
    pub fn discover_and_load() -> (Self, Option<PathBuf>) {
        if let Some(path) = discover_config_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                match parse_app_config(&content) {
                    Ok((cfg, fixed_lines)) => {
                        if !fixed_lines.is_empty() {
                            crate::diagnostics::log_raw(&format!(
                                "[WinPie:CONFIG] Auto-repaired unescaped Windows path(s) at line(s) {:?} in {}",
                                fixed_lines,
                                path.display()
                            ));
                        }
                        return (cfg, Some(path));
                    }
                    Err(e) => {
                        eprintln!("[WinPie] Error parsing YAML from {}: {}", path.display(), e);
                        crate::diagnostics::log_raw(&format!(
                            "[WinPie] Error parsing YAML from {}: {}",
                            path.display(),
                            e
                        ));
                        return (Self::default(), Some(path));
                    }
                }
            }
        }
        (Self::default(), None)
    }

    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        let p = path.as_ref();
        if p.as_os_str().is_empty() {
            return Self::discover_and_load().0;
        }
        if let Ok(content) = fs::read_to_string(p) {
            parse_app_config(&content).map(|(cfg, _)| cfg).unwrap_or_default()
        } else {
            Self::discover_and_load().0
        }
    }
}
