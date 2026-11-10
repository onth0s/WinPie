use serde::Deserialize;
use std::collections::HashMap;
use crate::geometry::Sector;
use super::matcher::ContextMatcher;

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

/// Canonical unified CommandNode for arbitrary-depth hierarchical command graph topologies (Phase 6 / 8).
/// Every node possesses the identical shape:
/// - `is_executable()`: true if `command` is non-empty.
/// - `is_branching()`: true if `children` is non-empty.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct CommandNode {
    pub label: String,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub children: HashMap<char, CommandNode>,
}

impl CommandNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn with_command(mut self, cmd: impl Into<String>) -> Self {
        self.command = Some(cmd.into());
        self
    }

    pub fn with_child(mut self, key: char, child: CommandNode) -> Self {
        self.children.insert(key, child);
        self
    }

    pub fn is_executable(&self) -> bool {
        self.command.as_ref().map(|c| !c.trim().is_empty()).unwrap_or(false)
    }

    pub fn is_branching(&self) -> bool {
        !self.children.is_empty()
    }
}

impl From<&MenuItem> for CommandNode {
    fn from(item: &MenuItem) -> Self {
        let mut node = CommandNode {
            label: item.label.clone(),
            tooltip: item.tooltip.clone(),
            icon: None,
            command: item.command.clone(),
            children: HashMap::new(),
        };
        if let Some(ref m) = item.menu {
            for (&k, child_item) in &m.items {
                node.children.insert(k, CommandNode::from(child_item));
            }
        }
        node
    }
}

impl From<&CommandNode> for MenuItem {
    fn from(node: &CommandNode) -> Self {
        let menu = if !node.children.is_empty() {
            let mut items = HashMap::new();
            for (&k, child) in &node.children {
                items.insert(k, MenuItem::from(child));
            }
            Some(MenuDefinition {
                title: node.label.clone(),
                tooltip: node.tooltip.clone(),
                items,
            })
        } else {
            None
        };

        MenuItem {
            label: node.label.clone(),
            tooltip: node.tooltip.clone(),
            command: node.command.clone(),
            menu,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProfileConfig {
    pub name: String,
    #[serde(default, alias = "when")]
    pub match_rules: ContextMatcher,
    #[serde(default)]
    pub wheel: WheelConfig,
    #[serde(default)]
    pub theme: Option<ThemeConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub daemon: DaemonConfig,
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

/// Daemon process behaviour configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DaemonConfig {
    /// When true, WinPie re-launches itself with Administrator privileges
    /// on startup (Windows ShellExecute runas verb). Allows the low-level
    /// keyboard hook to fire even when elevated windows (Task Manager,
    /// elevated terminals, etc.) have foreground focus. Default: false.
    #[serde(default)]
    pub run_as_admin: bool,
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
pub struct OverlayWindowConfig {
    #[serde(default = "default_true")]
    pub popup: bool,
    #[serde(default = "default_true")]
    pub toolwindow: bool,
    #[serde(default = "default_true")]
    pub topmost: bool,
    #[serde(default = "default_true")]
    pub no_activate: bool,
    #[serde(default = "default_true")]
    pub layered: bool,
    #[serde(default = "default_true")]
    pub transparent_input: bool,
}

impl Default for OverlayWindowConfig {
    fn default() -> Self {
        Self {
            popup: true,
            toolwindow: true,
            topmost: true,
            no_activate: true,
            layered: true,
            transparent_input: true,
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
    #[serde(default)]
    pub window: OverlayWindowConfig,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            radius: default_radius(),
            deadzone: default_deadzone(),
            coordinate_space: default_coord_space(),
            window: OverlayWindowConfig::default(),
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
    #[serde(default)]
    pub nodes: HashMap<String, CommandNode>,
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
            nodes: HashMap::new(),
        }
    }
}

impl WheelConfig {
    pub fn get_label(&self, sector: Sector) -> Option<&str> {
        let name = sector.name();
        if let Some(lbl) = self.labels.get(name) {
            Some(lbl.as_str())
        } else if let Some(m) = self.menus.get(name) {
            Some(m.title.as_str())
        } else if let Some(node) = self.nodes.get(name) {
            Some(node.label.as_str())
        } else {
            None
        }
    }

    pub fn get_tooltip(&self, sector: Sector) -> Option<&str> {
        let name = sector.name();
        if let Some(tt) = self.tooltips.get(name) {
            Some(tt.as_str())
        } else if let Some(m) = self.menus.get(name) {
            m.tooltip.as_deref()
        } else if let Some(node) = self.nodes.get(name) {
            node.tooltip.as_deref()
        } else {
            None
        }
    }

    pub fn get_command(&self, sector: Sector) -> Option<&str> {
        let name = sector.name();
        if let Some(cmd) = self.commands.get(name) {
            Some(cmd.as_str())
        } else if let Some(node) = self.nodes.get(name) {
            node.command.as_deref()
        } else {
            None
        }
    }

    pub fn get_menu(&self, sector: Sector) -> Option<MenuDefinition> {
        let name = sector.name();
        if let Some(m) = self.menus.get(name) {
            Some(m.clone())
        } else if let Some(node) = self.nodes.get(name) {
            if node.is_branching() {
                let item = MenuItem::from(node);
                item.menu
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_node(&self, sector: Sector) -> Option<CommandNode> {
        let name = sector.name();
        if let Some(node) = self.nodes.get(name) {
            return Some(node.clone());
        }
        if let Some(menu) = self.menus.get(name) {
            let mut items = HashMap::new();
            for (&k, item) in &menu.items {
                items.insert(k, CommandNode::from(item));
            }
            return Some(CommandNode {
                label: menu.title.clone(),
                tooltip: menu.tooltip.clone(),
                icon: None,
                command: None,
                children: items,
            });
        }
        if let Some(cmd) = self.commands.get(name) {
            let label = self.labels.get(name).cloned().unwrap_or_else(|| name.to_string());
            let tooltip = self.tooltips.get(name).cloned();
            return Some(CommandNode {
                label,
                tooltip,
                icon: None,
                command: Some(cmd.clone()),
                children: HashMap::new(),
            });
        }
        None
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

pub(crate) fn default_modifier() -> String { "win".to_string() }
pub(crate) fn default_trigger() -> String { "escape".to_string() }
pub(crate) fn default_coord_space() -> String { "physical_screen_pixels".to_string() }
pub(crate) fn default_true() -> bool { true }
pub(crate) fn default_radius() -> f64 { 180.0 }
pub(crate) fn default_deadzone() -> f64 { 32.0 }
pub(crate) fn default_slices() -> usize { 8 }
pub(crate) fn default_rotation() -> f64 { 0.0 }
pub(crate) fn default_font_family() -> String { "Segoe UI".to_string() }
pub(crate) fn default_font_size() -> i32 { 13 }
pub(crate) fn default_font_weight() -> i32 { 600 }
pub(crate) fn default_radius_ratio() -> f64 { 0.62 }
pub(crate) fn default_toast_duration() -> u32 { 1000 }
pub(crate) fn default_toast_corner() -> ToastCorner { ToastCorner::TopRight }
pub(crate) fn default_toast_margin() -> i32 { 24 }
pub(crate) fn default_toast_font_size() -> i32 { 13 }
pub(crate) fn default_corner_radius() -> f64 { 4.0 }

pub(crate) fn default_startup_toast_duration() -> u32 { 5000 }
pub(crate) fn default_startup_toast_text() -> String { "WinPie is active (Press Win+Esc)".to_string() }

pub(crate) fn default_hint_toast_corner() -> ToastCorner { ToastCorner::BottomRight }
pub(crate) fn default_hint_toast_font_size() -> i32 { 11 }
pub(crate) fn default_hint_toast_text() -> String { "[Tab] Loop   [Shift+Tab] Back   [Esc / RMB] Cancel".to_string() }

pub(crate) fn default_opacity_one() -> f64 { 1.0 }
pub(crate) fn default_accent_color() -> String { "#00AFFF".to_string() }
pub(crate) fn default_main_bg_color() -> String { "#14161C".to_string() }
pub(crate) fn default_main_bg_opacity() -> f64 { 0.94 }
pub(crate) fn default_secondary_bg_color() -> String { "#1E222B".to_string() }
pub(crate) fn default_secondary_bg_opacity() -> f64 { 0.80 }
pub(crate) fn default_border_color() -> String { "#3C465A".to_string() }
pub(crate) fn default_border_opacity() -> f64 { 0.85 }
pub(crate) fn default_text_primary() -> String { "#FFFFFF".to_string() }
pub(crate) fn default_text_secondary() -> String { "#A0A5B5".to_string() }
pub(crate) fn default_text_accent() -> String { "#00AFFF".to_string() }

pub(crate) fn default_spoke_color() -> String { "#EBEBEB".to_string() }
pub(crate) fn default_spoke_opacity() -> f64 { 0.75 }
pub(crate) fn default_rim_color() -> String { "#F0F0F0".to_string() }
pub(crate) fn default_rim_opacity() -> f64 { 0.86 }
pub(crate) fn default_hover_glow_color() -> String { "#00AFFF".to_string() }
pub(crate) fn default_hover_glow_opacity() -> f64 { 0.86 }
pub(crate) fn default_sector_bg_color() -> String { "#16181E".to_string() }
pub(crate) fn default_sector_bg_opacity() -> f64 { 0.59 }
pub(crate) fn default_deadzone_bg_color() -> String { "#141414".to_string() }
pub(crate) fn default_deadzone_bg_opacity() -> f64 { 0.51 }
pub(crate) fn default_deadzone_border_color() -> String { "#F0F0F0".to_string() }
pub(crate) fn default_deadzone_border_opacity() -> f64 { 0.86 }

impl AppConfig {
    /// Finds the index of the best matching profile with highest specificity score.
    pub fn find_matching_profile(&self, ctx: &crate::context::WindowContext) -> Option<usize> {
        let mut best: Option<(usize, u32)> = None;
        for (i, p) in self.profiles.iter().enumerate() {
            if p.match_rules.matches(ctx) {
                let score = p.match_rules.specificity_score();
                match best {
                    Some((_, best_score)) if score > best_score => {
                        best = Some((i, score));
                    }
                    None => {
                        best = Some((i, score));
                    }
                    _ => {}
                }
            }
        }
        best.map(|(idx, _)| idx)
    }

    /// Finds the best matching profile evaluated against the frozen InvocationContext.
    pub fn find_matching_profile_for_invocation(&self, inv: &crate::context::InvocationContext) -> Option<usize> {
        let mut best: Option<(usize, u32)> = None;
        for (i, p) in self.profiles.iter().enumerate() {
            if p.match_rules.matches_invocation(inv) {
                let score = p.match_rules.specificity_score();
                match best {
                    Some((_, best_score)) if score > best_score => {
                        best = Some((i, score));
                    }
                    None => {
                        best = Some((i, score));
                    }
                    _ => {}
                }
            }
        }
        best.map(|(idx, _)| idx)
    }

    pub fn get_label_for_sector(&self, sector: Sector) -> &str {
        self.get_label_for_sector_with_profile(sector, None)
    }

    pub fn get_label_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> &str {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(lbl) = profile.wheel.get_label(sector) {
                    return lbl;
                }
            }
        }
        self.wheel.get_label(sector).unwrap_or(sector.name())
    }

    pub fn get_tooltip_for_sector(&self, sector: Sector) -> Option<&str> {
        self.get_tooltip_for_sector_with_profile(sector, None)
    }

    pub fn get_tooltip_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> Option<&str> {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(tt) = profile.wheel.get_tooltip(sector) {
                    return Some(tt);
                }
            }
        }
        self.wheel.get_tooltip(sector)
    }

    pub fn get_command_for_sector(&self, sector: Sector) -> Option<&str> {
        self.get_command_for_sector_with_profile(sector, None)
    }

    pub fn get_command_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> Option<&str> {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(cmd) = profile.wheel.get_command(sector) {
                    return Some(cmd);
                }
            }
        }
        self.wheel.get_command(sector)
    }

    pub fn get_menu_for_sector(&self, sector: Sector) -> Option<MenuDefinition> {
        self.get_menu_for_sector_with_profile(sector, None)
    }

    pub fn get_menu_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> Option<MenuDefinition> {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(m) = profile.wheel.get_menu(sector) {
                    return Some(m);
                }
            }
        }
        self.wheel.get_menu(sector)
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

    pub fn get_node_for_sector(&self, sector: Sector) -> Option<CommandNode> {
        self.get_node_for_sector_with_profile(sector, None)
    }

    pub fn get_node_for_sector_with_profile(&self, sector: Sector, profile_idx: Option<usize>) -> Option<CommandNode> {
        if let Some(idx) = profile_idx {
            if let Some(profile) = self.profiles.get(idx) {
                if let Some(node) = profile.wheel.get_node(sector) {
                    return Some(node);
                }
            }
        }
        self.wheel.get_node(sector)
    }
}
