use crate::config::DiagnosticsConfig;
use crate::geometry::{Point, Sector};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn get_log_path() -> PathBuf {
    std::env::temp_dir().join("winpie_debug.log")
}

pub fn log_raw(msg: &str) {
    println!("{}", msg);

    if let Ok(mut lock) = LOG_FILE.lock() {
        if lock.is_none() {
            let log_path = get_log_path();
            // Bound debug log file to at most 5MB
            if let Ok(meta) = std::fs::metadata(&log_path) {
                if meta.len() > 5 * 1024 * 1024 {
                    let _ = std::fs::remove_file(&log_path);
                }
            }
            if let Ok(file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                *lock = Some(file);
            }
        }

        if let Some(ref mut file) = *lock {
            let _ = writeln!(file, "{}", msg);
            let _ = file.flush();
        }
    }
}

pub struct Diagnostics {
    pub config: DiagnosticsConfig,
}

impl Diagnostics {
    pub fn new(config: DiagnosticsConfig) -> Self {
        Self { config }
    }

    pub fn log_activate(&self, pt: Point) {
        if self.config.enabled {
            log_raw(&format!("[WinPie:DIAGNOSTIC] Activated at ({}, {})", pt.x, pt.y));
        }
    }

    pub fn log_hover(&self, sector: Option<Sector>) {
        if self.config.enabled {
            if let Some(s) = sector {
                log_raw(&format!("[WinPie:DIAGNOSTIC] Hover: {}", s.name()));
            } else {
                log_raw("[WinPie:DIAGNOSTIC] Hover: DEADZONE (None)");
            }
        }
    }

    pub fn log_commit(&self, sector: Sector) {
        if self.config.enabled && self.config.log_selection {
            log_raw(&format!("[WinPie:DIAGNOSTIC] Committed sector: {}", sector.name()));
        }
    }

    pub fn log_cancel(&self) {
        if self.config.enabled && self.config.log_cancel {
            log_raw("[WinPie:DIAGNOSTIC] Cancelled");
        }
    }

    pub fn log_error(&self, msg: &str) {
        if self.config.enabled && self.config.log_fatal_errors {
            log_raw(&format!("[WinPie:ERROR] {}", msg));
        }
    }
}
