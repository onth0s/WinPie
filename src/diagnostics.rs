use crate::config::DiagnosticsConfig;
use crate::geometry::{Point, Sector};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn get_log_path() -> PathBuf {
    std::env::temp_dir().join("winpie_debug.log")
}

pub fn log_raw(msg: &str) {
    println!("{}", msg);

    if let Ok(_lock) = LOG_MUTEX.lock() {
        let log_path = get_log_path();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
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
