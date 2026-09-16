use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::geometry::Sector;

#[derive(Debug, Clone, Deserialize, Default)]
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
            modifier: default_modifier(),
            trigger: default_trigger(),
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
            radius: default_radius(),
            deadzone: default_deadzone(),
            coordinate_space: default_coord_space(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WheelConfig {
    #[serde(default = "default_slices")]
    pub slices: usize,
    #[serde(default = "default_rotation")]
    pub rotation_degrees: f64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            slices: default_slices(),
            rotation_degrees: default_rotation(),
            labels: HashMap::new(),
            commands: HashMap::new(),
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
            family: default_font_family(),
            size: default_font_size(),
            weight: default_font_weight(),
            radius_ratio: default_radius_ratio(),
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
            duration_ms: default_toast_duration(),
            corner: default_toast_corner(),
            margin_x: default_toast_margin(),
            margin_y: default_toast_margin(),
            font_size: default_toast_font_size(),
            show_sector_direction: true,
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
fn default_font_family() -> String { "Segoe UI".to_string() }
fn default_font_size() -> i32 { 13 }
fn default_font_weight() -> i32 { 600 }
fn default_radius_ratio() -> f64 { 0.62 }
fn default_toast_duration() -> u32 { 1000 }
fn default_toast_corner() -> ToastCorner { ToastCorner::BottomRight }
fn default_toast_margin() -> i32 { 24 }
fn default_toast_font_size() -> i32 { 13 }

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

    pub fn get_command_for_sector(&self, sector: Sector) -> Option<&str> {
        self.wheel.commands.get(sector.name()).map(|s| s.as_str())
    }
}
