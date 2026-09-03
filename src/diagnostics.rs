use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub activation: ActivationConfig,
    #[serde(default)]
    pub overlay: OverlayConfig,
    #[serde(default)]
    pub wheel: WheelConfig,
    #[serde(default)]
    pub rendering: RenderingConfig,
    #[serde(default)]
    pub diagnostics: DiagnosticsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActivationConfig {
    #[serde(default = "default_modifier")]
    pub modifier: String,
    #[serde(default = "default_trigger")]
    pub trigger: String,
    #[serde(default = "default_true")]
    pub swallow_trigger: bool,
    #[serde(default = "default_true")]
    pub left_win: bool,
    #[serde(default = "default_true")]
    pub right_win: bool,
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            modifier: "win".to_string(),
            trigger: "escape".to_string(),
            swallow_trigger: true,
            left_win: true,
            right_win: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OverlayConfig {
    #[serde(default = "default_radius")]
    pub radius: f64,
    #[serde(default = "default_deadzone")]
    pub deadzone: f64,
    #[serde(default = "default_coord_space")]
    pub coordinate_space: String,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            radius: 180.0,
            deadzone: 32.0,
            coordinate_space: "physical_screen_pixels".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WheelConfig {
    #[serde(default = "default_slices")]
    pub slices: usize,
    #[serde(default = "default_rotation")]
    pub rotation_degrees: f64,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            slices: 8,
            rotation_degrees: 0.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenderingConfig {
    #[serde(default = "default_true")]
    pub show_stubs: bool,
    #[serde(default = "default_true")]
    pub show_deadzone: bool,
    #[serde(default = "default_true")]
    pub highlight_hovered: bool,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            show_stubs: true,
            show_deadzone: true,
            highlight_hovered: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiagnosticsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub log_selection: bool,
    #[serde(default = "default_true")]
    pub log_cancel: bool,
    #[serde(default = "default_true")]
    pub log_fatal_errors: bool,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_selection: true,
            log_cancel: true,
            log_fatal_errors: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            activation: ActivationConfig::default(),
            overlay: OverlayConfig::default(),
            wheel: WheelConfig::default(),
            rendering: RenderingConfig::default(),
            diagnostics: DiagnosticsConfig::default(),
        }
    }
}

fn default_modifier() -> String { "win".to_string() }
fn default_trigger() -> String { "escape".to_string() }
fn default_coord_space() -> String { "physical_screen_pixels".to_string() }
fn default_true() -> bool { true }
fn default_radius() -> f64 { 180.0 }
fn default_deadzone() -> f64 { 32.0 }
fn default_slices() -> usize { 8 }
fn default_rotation() -> f64 { 0.0 }

impl AppConfig {
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            serde_yaml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
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

    pub fn log_activate(&self, pt: crate::geometry::Point) {
        if self.config.enabled {
            println!("[WinPie:DIAGNOSTIC] Activated at ({}, {})", pt.x, pt.y);
        }
    }

    pub fn log_hover(&self, sector: Option<crate::geometry::Sector>) {
        if self.config.enabled {
            if let Some(s) = sector {
                println!("[WinPie:DIAGNOSTIC] Hover: {}", s.name());
            } else {
                println!("[WinPie:DIAGNOSTIC] Hover: DEADZONE (None)");
            }
        }
    }

    pub fn log_commit(&self, sector: crate::geometry::Sector) {
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
