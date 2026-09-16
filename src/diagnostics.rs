use crate::config::DiagnosticsConfig;
use crate::geometry::{Point, Sector};

pub struct Diagnostics {
    pub config: DiagnosticsConfig,
}

impl Diagnostics {
    pub fn new(config: DiagnosticsConfig) -> Self {
        Self { config }
    }

    pub fn log_activate(&self, pt: Point) {
        if self.config.enabled {
            println!("[WinPie:DIAGNOSTIC] Activated at ({}, {})", pt.x, pt.y);
        }
    }

    pub fn log_hover(&self, sector: Option<Sector>) {
        if self.config.enabled {
            if let Some(s) = sector {
                println!("[WinPie:DIAGNOSTIC] Hover: {}", s.name());
            } else {
                println!("[WinPie:DIAGNOSTIC] Hover: DEADZONE (None)");
            }
        }
    }

    pub fn log_commit(&self, sector: Sector) {
        if self.config.enabled && self.config.log_selection {
            println!("[WinPie:DIAGNOSTIC] Committed sector: {}", sector.name());
        }
    }

    pub fn log_cancel(&self) {
        if self.config.enabled && self.config.log_cancel {
            println!("[WinPie:DIAGNOSTIC] Cancelled");
        }
    }

    pub fn log_error(&self, msg: &str) {
        if self.config.enabled && self.config.log_fatal_errors {
            eprintln!("[WinPie:ERROR] {}", msg);
        }
    }
}
