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

/// Represents a single line-level YAML auto-repair performed by WinPie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlRepairChange {
    pub line_number: usize,
    pub original: String,
    pub repaired: String,
    pub reason: String,
}

/// Result of sanitizing a YAML configuration string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlRepairResult {
    pub sanitized: String,
    pub changes: Vec<YamlRepairChange>,
}

/// Formats a filesystem path for human-readable diagnostics by stripping the Windows `\\?\` verbatim prefix.
pub fn format_display_path(path: &Path) -> String {
    let s = path.display().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

/// Formats a comprehensive, human-readable summary of YAML auto-repairs.
pub fn format_repair_summary(path: &Path, changes: &[YamlRepairChange], persisted: bool) -> String {
    let display_path = format_display_path(path);
    let mode_note = if persisted {
        "persisted to disk"
    } else {
        "in-memory only; file on disk was not modified"
    };

    let mut out = format!(
        "[WinPie:CONFIG] Auto-repaired {} unescaped Windows path(s) in {} ({}):",
        changes.len(),
        display_path,
        mode_note
    );

    for c in changes {
        out.push_str(&format!(
            "\n  Line {}: {}\n    - before: {}\n    + after : {}",
            c.line_number, c.reason, c.original, c.repaired
        ));
    }

    if !persisted {
        out.push_str("\n  Tip: Run `winpie --repair` to write these fixes to disk permanently (or use single quotes '...').");
    }

    out
}

/// Automatically sanitizes common YAML string escaping issues, particularly Windows paths
/// with unescaped backslashes inside double quotes (e.g. "C:\Users\..." triggering \U escape errors).
/// Note: Single-quoted YAML strings ('...') already treat backslashes literally and are never modified.
pub fn sanitize_yaml_string_detailed(input: &str) -> YamlRepairResult {
    let mut changes = Vec::new();
    let mut output_lines = Vec::new();
    let line_ending = if input.contains("\r\n") { "\r\n" } else { "\n" };

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
                            let repaired_line = format!("{}{}{}", before, sanitized_inside, after);
                            changes.push(YamlRepairChange {
                                line_number: line_num,
                                original: line.trim().to_string(),
                                repaired: repaired_line.trim().to_string(),
                                reason: "Escaped single backslashes (\\ -> \\\\) in double-quoted Windows path".to_string(),
                            });
                            output_lines.push(repaired_line);
                            continue;
                        }
                    }
                }
            }
        }
        output_lines.push(line.to_string());
    }

    let mut result = output_lines.join(line_ending);
    if input.ends_with('\n') {
        result.push_str(line_ending);
    }
    YamlRepairResult {
        sanitized: result,
        changes,
    }
}

/// Automatically sanitizes common YAML string escaping issues, returning `(sanitized_yaml, fixed_line_numbers)`.
pub fn sanitize_yaml_string(input: &str) -> (String, Vec<usize>) {
    let res = sanitize_yaml_string_detailed(input);
    let fixed_lines = res.changes.iter().map(|c| c.line_number).collect();
    (res.sanitized, fixed_lines)
}

/// Parses YAML string into AppConfig with automatic unescaped Windows path repair.
/// Returns `Ok((AppConfig, YamlRepairResult))` or `Err(detailed_error_message)`.
pub fn parse_app_config_detailed(content: &str) -> Result<(AppConfig, YamlRepairResult), String> {
    // 1. First attempt direct parse
    match serde_yaml::from_str::<AppConfig>(content) {
        Ok(cfg) => Ok((
            cfg,
            YamlRepairResult {
                sanitized: content.to_string(),
                changes: Vec::new(),
            },
        )),
        Err(orig_err) => {
            // 2. Attempt auto-repair on failure
            let repair = sanitize_yaml_string_detailed(content);
            if !repair.changes.is_empty() {
                if let Ok(cfg) = serde_yaml::from_str::<AppConfig>(&repair.sanitized) {
                    return Ok((cfg, repair));
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

/// Parses YAML string into AppConfig with automatic unescaped Windows path repair.
/// Returns Ok((AppConfig, auto_fixed_lines)) or Err(detailed_error_message).
pub fn parse_app_config(content: &str) -> Result<(AppConfig, Vec<usize>), String> {
    parse_app_config_detailed(content).map(|(cfg, repair)| {
        let lines = repair.changes.iter().map(|c| c.line_number).collect();
        (cfg, lines)
    })
}

/// Validates and persistently repairs unescaped Windows paths in the given YAML configuration file.
pub fn repair_config_file(path: &Path) -> Result<YamlRepairResult, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", format_display_path(path), e))?;

    let (_cfg, repair) = parse_app_config_detailed(&content)?;
    if !repair.changes.is_empty() {
        fs::write(path, &repair.sanitized)
            .map_err(|e| format!("Failed to write {}: {}", format_display_path(path), e))?;
    }
    Ok(repair)
}

impl AppConfig {
    /// Discovers config file path and loads it. Returns (AppConfig, Option<PathBuf>).
    pub fn discover_and_load() -> (Self, Option<PathBuf>) {
        if let Some(path) = discover_config_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                match parse_app_config_detailed(&content) {
                    Ok((cfg, repair)) => {
                        if !repair.changes.is_empty() {
                            crate::diagnostics::log_raw(&format_repair_summary(
                                &path,
                                &repair.changes,
                                false,
                            ));
                        }
                        return (cfg, Some(path));
                    }
                    Err(e) => {
                        eprintln!("[WinPie] Error parsing YAML from {}: {}", format_display_path(&path), e);
                        crate::diagnostics::log_raw(&format!(
                            "[WinPie] Error parsing YAML from {}: {}",
                            format_display_path(&path),
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
