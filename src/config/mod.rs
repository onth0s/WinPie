use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::geometry::Sector;

/// Helper to parse a hex color string ("#RRGGBB" or "RRGGBB") into RGB components in 0.0..=255.0
pub fn parse_hex_color(s: &str) -> (f64, f64, f64) {
    let clean = s.trim().trim_start_matches('#');
    if clean.len() >= 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255) as f64;
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255) as f64;
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255) as f64;
        (r, g, b)
    } else {
        (255.0, 255.0, 255.0)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct MenuItem {
    pub label: String,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub menu: Option<MenuDefinition>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct MenuDefinition {
    pub title: String,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub items: HashMap<char, MenuItem>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct ContextMatcher {
    #[serde(default)]
    pub process: Option<String>,
    #[serde(default)]
    pub window_class: Option<String>,
    #[serde(default)]
    pub window_title: Option<String>,
}

impl ContextMatcher {
    /// Returns true if the window context matches all specified non-empty criteria.
    pub fn matches(&self, ctx: &crate::context::WindowContext) -> bool {
        if let Some(target_proc) = &self.process {
            if !target_proc.is_empty() {
                let p_clean = target_proc.trim().to_lowercase();
                if ctx.process_name != p_clean && !ctx.process_name.contains(&p_clean) {
                    return false;
                }
            }
        }

        if let Some(target_class) = &self.window_class {
            if !target_class.is_empty() && !ctx.window_class.eq_ignore_ascii_case(target_class.trim()) {
                return false;
            }
        }

        if let Some(target_title) = &self.window_title {
            if !target_title.is_empty() {
                let t_clean = target_title.to_lowercase();
                if !ctx.window_title.to_lowercase().contains(&t_clean) {
                    return false;
                }
            }
        }

        // If at least one criteria was defined and matched
        self.process.is_some() || self.window_class.is_some() || self.window_title.is_some()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProfileConfig {
    pub name: String,
    #[serde(default)]
    pub match_rules: ContextMatcher,
    #[serde(default)]
    pub wheel: WheelConfig,
    #[serde(default)]
    pub theme: Option<ThemeConfig>,
}

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
    #[serde(default)]
    pub startup_toast: StartupToastConfig,
    #[serde(default)]
    pub hint_toast: HintToastConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub profiles: Vec<ProfileConfig>,
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
    pub tooltips: HashMap<String, String>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
    #[serde(default)]
    pub menus: HashMap<String, MenuDefinition>,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            slices: default_slices(),
            rotation_degrees: default_rotation(),
            labels: HashMap::new(),
            tooltips: HashMap::new(),
            commands: HashMap::new(),
            menus: HashMap::new(),
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
    #[serde(default = "default_true")]
    pub show_hover_tooltips: bool,
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
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_true")]
    pub show_sector_direction: bool,
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_hover_tooltips: true,
            duration_ms: default_toast_duration(),
            corner: default_toast_corner(),
            margin_x: default_toast_margin(),
            margin_y: default_toast_margin(),
            font_size: default_toast_font_size(),
            corner_radius: default_corner_radius(),
            show_sector_direction: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartupToastConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_startup_toast_duration")]
    pub duration_ms: u32,
    #[serde(default = "default_toast_corner")]
    pub corner: ToastCorner,
    #[serde(default = "default_toast_margin")]
    pub margin_x: i32,
    #[serde(default = "default_toast_margin")]
    pub margin_y: i32,
    #[serde(default = "default_toast_font_size")]
    pub font_size: i32,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_startup_toast_text")]
    pub text: String,
}

impl Default for StartupToastConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_ms: default_startup_toast_duration(),
            corner: default_toast_corner(),
            margin_x: default_toast_margin(),
            margin_y: default_toast_margin(),
            font_size: default_toast_font_size(),
            corner_radius: default_corner_radius(),
            text: default_startup_toast_text(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HintToastConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_hint_toast_corner")]
    pub corner: ToastCorner,
    #[serde(default = "default_toast_margin")]
    pub margin_x: i32,
    #[serde(default = "default_toast_margin")]
    pub margin_y: i32,
    #[serde(default = "default_hint_toast_font_size")]
    pub font_size: i32,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default = "default_hint_toast_text")]
    pub text: String,
}

impl Default for HintToastConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            corner: default_hint_toast_corner(),
            margin_x: default_toast_margin(),
            margin_y: default_toast_margin(),
            font_size: default_hint_toast_font_size(),
            corner_radius: default_corner_radius(),
            text: default_hint_toast_text(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_corner_radius")]
    pub menu_corner_radius: f64,
    #[serde(default = "default_corner_radius")]
    pub toast_corner_radius: f64,

    #[serde(default = "default_accent_color")]
    pub accent_color: String,
    #[serde(default = "default_opacity_one")]
    pub accent_opacity: f64,

    #[serde(default = "default_main_bg_color")]
    pub main_bg_color: String,
    #[serde(default = "default_main_bg_opacity")]
    pub main_bg_opacity: f64,

    #[serde(default = "default_secondary_bg_color")]
    pub secondary_bg_color: String,
    #[serde(default = "default_secondary_bg_opacity")]
    pub secondary_bg_opacity: f64,

    #[serde(default = "default_border_color")]
    pub border_color: String,
    #[serde(default = "default_border_opacity")]
    pub border_opacity: f64,

    #[serde(default = "default_text_primary")]
    pub text_primary: String,
    #[serde(default = "default_text_secondary")]
    pub text_secondary: String,
    #[serde(default = "default_text_accent")]
    pub text_accent: String,

    #[serde(default)]
    pub wheel: WheelThemeConfig,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            menu_corner_radius: default_corner_radius(),
            toast_corner_radius: default_corner_radius(),
            accent_color: default_accent_color(),
            accent_opacity: default_opacity_one(),
            main_bg_color: default_main_bg_color(),
            main_bg_opacity: default_main_bg_opacity(),
            secondary_bg_color: default_secondary_bg_color(),
            secondary_bg_opacity: default_secondary_bg_opacity(),
            border_color: default_border_color(),
            border_opacity: default_border_opacity(),
            text_primary: default_text_primary(),
            text_secondary: default_text_secondary(),
            text_accent: default_text_accent(),
            wheel: WheelThemeConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WheelThemeConfig {
    #[serde(default = "default_spoke_color")]
    pub spoke_color: String,
    #[serde(default = "default_spoke_opacity")]
    pub spoke_opacity: f64,

    #[serde(default = "default_rim_color")]
    pub rim_color: String,
    #[serde(default = "default_rim_opacity")]
    pub rim_opacity: f64,

    #[serde(default = "default_hover_glow_color")]
    pub hover_glow_color: String,
    #[serde(default = "default_hover_glow_opacity")]
    pub hover_glow_opacity: f64,

    #[serde(default = "default_sector_bg_color")]
    pub sector_bg_color: String,
    #[serde(default = "default_sector_bg_opacity")]
    pub sector_bg_opacity: f64,

    #[serde(default = "default_deadzone_bg_color")]
    pub deadzone_bg_color: String,
    #[serde(default = "default_deadzone_bg_opacity")]
    pub deadzone_bg_opacity: f64,

    #[serde(default = "default_deadzone_border_color")]
    pub deadzone_border_color: String,
    #[serde(default = "default_deadzone_border_opacity")]
    pub deadzone_border_opacity: f64,
}

impl Default for WheelThemeConfig {
    fn default() -> Self {
        Self {
            spoke_color: default_spoke_color(),
            spoke_opacity: default_spoke_opacity(),
            rim_color: default_rim_color(),
            rim_opacity: default_rim_opacity(),
            hover_glow_color: default_hover_glow_color(),
            hover_glow_opacity: default_hover_glow_opacity(),
            sector_bg_color: default_sector_bg_color(),
            sector_bg_opacity: default_sector_bg_opacity(),
            deadzone_bg_color: default_deadzone_bg_color(),
            deadzone_bg_opacity: default_deadzone_bg_opacity(),
            deadzone_border_color: default_deadzone_border_color(),
            deadzone_border_opacity: default_deadzone_border_opacity(),
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
fn default_toast_corner() -> ToastCorner { ToastCorner::TopRight }
fn default_toast_margin() -> i32 { 24 }
fn default_toast_font_size() -> i32 { 13 }
fn default_corner_radius() -> f64 { 4.0 }

fn default_startup_toast_duration() -> u32 { 5000 }
fn default_startup_toast_text() -> String { "WinPie is active (Press Win+Esc)".to_string() }

fn default_hint_toast_corner() -> ToastCorner { ToastCorner::BottomRight }
fn default_hint_toast_font_size() -> i32 { 11 }
fn default_hint_toast_text() -> String { "[Tab] Loop   [Shift+Tab] Back   [Esc / RMB] Cancel".to_string() }

fn default_opacity_one() -> f64 { 1.0 }
fn default_accent_color() -> String { "#00AFFF".to_string() }
fn default_main_bg_color() -> String { "#14161C".to_string() }
fn default_main_bg_opacity() -> f64 { 0.94 }
fn default_secondary_bg_color() -> String { "#1E222B".to_string() }
fn default_secondary_bg_opacity() -> f64 { 0.80 }
fn default_border_color() -> String { "#3C465A".to_string() }
fn default_border_opacity() -> f64 { 0.85 }
fn default_text_primary() -> String { "#FFFFFF".to_string() }
fn default_text_secondary() -> String { "#A0A5B5".to_string() }
fn default_text_accent() -> String { "#00AFFF".to_string() }

fn default_spoke_color() -> String { "#EBEBEB".to_string() }
fn default_spoke_opacity() -> f64 { 0.75 }
fn default_rim_color() -> String { "#F0F0F0".to_string() }
fn default_rim_opacity() -> f64 { 0.86 }
fn default_hover_glow_color() -> String { "#00AFFF".to_string() }
fn default_hover_glow_opacity() -> f64 { 0.86 }
fn default_sector_bg_color() -> String { "#16181E".to_string() }
fn default_sector_bg_opacity() -> f64 { 0.59 }
fn default_deadzone_bg_color() -> String { "#141414".to_string() }
fn default_deadzone_bg_opacity() -> f64 { 0.51 }
fn default_deadzone_border_color() -> String { "#F0F0F0".to_string() }
fn default_deadzone_border_opacity() -> f64 { 0.86 }

impl AppConfig {
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            serde_yaml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Finds the index of the first profile matching the given WindowContext.
    pub fn find_matching_profile(&self, ctx: &crate::context::WindowContext) -> Option<usize> {
        for (i, p) in self.profiles.iter().enumerate() {
            if p.match_rules.matches(ctx) {
                return Some(i);
            }
        }
        None
    }

    pub fn get_label_for_sector(&self, sector: Sector) -> &str {
        self.get_label_for_sector_with_profile(sector, None)
    }

    pub fn get_label_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> &str {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(lbl) = profile.wheel.labels.get(sector.name()) {
                    return lbl.as_str();
                } else if let Some(m) = profile.wheel.menus.get(sector.name()) {
                    return m.title.as_str();
                }
            }
        }

        if let Some(lbl) = self.wheel.labels.get(sector.name()) {
            lbl.as_str()
        } else if let Some(m) = self.wheel.menus.get(sector.name()) {
            m.title.as_str()
        } else {
            sector.name()
        }
    }

    pub fn get_tooltip_for_sector(&self, sector: Sector) -> Option<&str> {
        self.get_tooltip_for_sector_with_profile(sector, None)
    }

    pub fn get_tooltip_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> Option<&str> {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(tt) = profile.wheel.tooltips.get(sector.name()) {
                    return Some(tt.as_str());
                } else if let Some(m) = profile.wheel.menus.get(sector.name()) {
                    return m.tooltip.as_deref();
                }
            }
        }

        if let Some(tt) = self.wheel.tooltips.get(sector.name()) {
            Some(tt.as_str())
        } else if let Some(m) = self.wheel.menus.get(sector.name()) {
            m.tooltip.as_deref()
        } else {
            None
        }
    }

    pub fn get_command_for_sector(&self, sector: Sector) -> Option<&str> {
        self.get_command_for_sector_with_profile(sector, None)
    }

    pub fn get_command_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> Option<&str> {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(cmd) = profile.wheel.commands.get(sector.name()) {
                    return Some(cmd.as_str());
                }
            }
        }
        self.wheel.commands.get(sector.name()).map(|s| s.as_str())
    }

    pub fn get_menu_for_sector(&self, sector: Sector) -> Option<&MenuDefinition> {
        self.get_menu_for_sector_with_profile(sector, None)
    }

    pub fn get_menu_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> Option<&MenuDefinition> {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(m) = profile.wheel.menus.get(sector.name()) {
                    return Some(m);
                }
            }
        }
        self.wheel.menus.get(sector.name())
    }

    pub fn get_theme_for_profile(&self, profile_idx: Option<usize>) -> &ThemeConfig {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(theme) = &profile.theme {
                    return theme;
                }
            }
        }
        &self.theme
    }
}
