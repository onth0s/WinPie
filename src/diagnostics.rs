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
    #[serde(default)]
    pub toast: ToastConfig,
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

use std::collections::HashMap;
use crate::geometry::Sector;

#[derive(Debug, Clone, Deserialize)]
pub struct WheelConfig {
    #[serde(default = "default_slices")]
    pub slices: usize,
    #[serde(default = "default_rotation")]
    pub rotation_degrees: f64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            slices: 8,
            rotation_degrees: 0.0,
            labels: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontConfig {
    #[serde(default = "default_font_family")]
    pub family: String,
    #[serde(default = "default_font_size")]
    pub size: i32,
    #[serde(default = "default_font_weight")]
    pub weight: i32,
    #[serde(default = "default_radius_ratio")]
    pub radius_ratio: f64,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Segoe UI".to_string(),
            size: 13,
            weight: 600,
            radius_ratio: 0.62,
        }
    }
}

fn default_font_family() -> String { "Segoe UI".to_string() }
fn default_font_size() -> i32 { 13 }
fn default_font_weight() -> i32 { 600 }
fn default_radius_ratio() -> f64 { 0.62 }

#[derive(Debug, Clone, Deserialize)]
pub struct RenderingConfig {
    #[serde(default = "default_true")]
    pub show_stubs: bool,
    #[serde(default = "default_true")]
    pub show_deadzone: bool,
    #[serde(default = "default_true")]
    pub highlight_hovered: bool,
    #[serde(default = "default_true")]
    pub show_labels: bool,
    #[serde(default)]
    pub font: FontConfig,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            show_stubs: true,
            show_deadzone: true,
            highlight_hovered: true,
            show_labels: true,
            font: FontConfig::default(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToastCorner {
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToastConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_toast_duration")]
    pub duration_ms: u32,
    #[serde(default = "default_toast_corner")]
    pub corner: ToastCorner,
    #[serde(default = "default_toast_margin")]
    pub margin_x: i32,
    #[serde(default = "default_toast_margin")]
    pub margin_y: i32,
    #[serde(default = "default_toast_font_size")]
    pub font_size: i32,
    #[serde(default = "default_true")]
    pub show_sector_direction: bool,
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_ms: 1000,
            corner: ToastCorner::BottomRight,
            margin_x: 24,
            margin_y: 24,
            font_size: 13,
            show_sector_direction: true,
        }
    }
}

fn default_toast_duration() -> u32 { 1000 }
fn default_toast_corner() -> ToastCorner { ToastCorner::BottomRight }
fn default_toast_margin() -> i32 { 24 }
fn default_toast_font_size() -> i32 { 13 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            activation: ActivationConfig::default(),
            overlay: OverlayConfig::default(),
            wheel: WheelConfig::default(),
            rendering: RenderingConfig::default(),
            diagnostics: DiagnosticsConfig::default(),
            toast: ToastConfig::default(),
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

    pub fn get_label_for_sector(&self, sector: Sector) -> &str {
        if let Some(lbl) = self.wheel.labels.get(sector.name()) {
            lbl.as_str()
        } else {
            sector.name()
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
