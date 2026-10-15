use std::fs;
use std::path::Path;
use winpie::config::{discover_config_path, AppConfig, ToastCorner};
use winpie::geometry::Sector;

#[test]
fn test_config_example_yaml_integrity_and_full_specification() {
    let example_path = Path::new("config.example.yaml");
    assert!(
        example_path.exists(),
        "config.example.yaml must exist bare on repository root"
    );

    let content = fs::read_to_string(example_path).expect("Read config.example.yaml");
    let config: AppConfig = serde_yaml::from_str(&content).expect("Parse config.example.yaml");

    // 1. Activation
    assert_eq!(config.activation.modifier, "win");
    assert_eq!(config.activation.trigger, "escape");
    assert!(config.activation.swallow_trigger);
    assert!(config.activation.left_win);
    assert!(config.activation.right_win);

    // 2. Overlay
    assert_eq!(config.overlay.radius, 180.0);
    assert_eq!(config.overlay.deadzone, 32.0);
    assert_eq!(config.overlay.coordinate_space, "physical_screen_pixels");

    // 3. Wheel & Sectors
    assert_eq!(config.wheel.slices, 8);
    assert_eq!(config.wheel.rotation_degrees, 0.0);
    assert_eq!(config.get_label_for_sector(Sector::N), "Dev Tools");
    assert_eq!(config.get_label_for_sector(Sector::NE), "Browser");
    assert_eq!(config.get_label_for_sector(Sector::E), "Sublime");
    assert_eq!(config.get_label_for_sector(Sector::SE), "Files");
    assert_eq!(config.get_label_for_sector(Sector::S), "Settings");
    assert_eq!(config.get_label_for_sector(Sector::SW), "Music");
    assert_eq!(config.get_label_for_sector(Sector::W), "Tasks");
    assert_eq!(config.get_label_for_sector(Sector::NW), "Chat");

    // Tooltips & Commands
    assert_eq!(
        config.get_tooltip_for_sector(Sector::NE),
        Some("Open default web browser")
    );
    assert_eq!(
        config.get_command_for_sector(Sector::E),
        Some("sublime.exe")
    );
    assert_eq!(
        config.get_command_for_sector(Sector::SE),
        Some("explorer.exe")
    );
    assert_eq!(
        config.get_command_for_sector(Sector::S),
        Some("control.exe")
    );
    assert_eq!(
        config.get_command_for_sector(Sector::W),
        Some("taskmgr.exe")
    );

    // Nested Modal Menus
    let menu_n = config.get_menu_for_sector(Sector::N).expect("Menu N");
    assert_eq!(menu_n.title, "Dev Tools");
    let item_a = menu_n.items.get(&'a').expect("Item 'a' in Menu N");
    assert_eq!(item_a.label, "VS Code");
    assert_eq!(item_a.command.as_deref(), Some("code.exe"));

    let item_d = menu_n.items.get(&'d').expect("Item 'd' in Menu N");
    assert_eq!(item_d.label, "Cargo Pipeline");
    let sub_d = item_d.menu.as_ref().expect("Submenu in item 'd'");
    assert_eq!(sub_d.title, "Cargo Pipeline");
    let clean_item = sub_d.items.get(&'c').expect("Cargo Clean item");
    assert_eq!(clean_item.command.as_deref(), Some("cargo clean"));

    // 4. Rendering & Typography
    assert!(config.rendering.show_stubs);
    assert!(config.rendering.show_deadzone);
    assert!(config.rendering.highlight_hovered);
    assert!(config.rendering.show_labels);
    assert_eq!(config.rendering.font.family, "Segoe UI");
    assert_eq!(config.rendering.font.size, 13);
    assert_eq!(config.rendering.font.weight, 600);
    assert_eq!(config.rendering.font.radius_ratio, 0.62);

    // 5. Diagnostics
    assert!(config.diagnostics.enabled);
    assert!(config.diagnostics.log_selection);
    assert!(config.diagnostics.log_cancel);
    assert!(config.diagnostics.log_fatal_errors);

    // 6. Toasts
    assert!(config.toast.enabled);
    assert!(config.toast.show_hover_tooltips);
    assert_eq!(config.toast.duration_ms, 1000);
    assert_eq!(config.toast.corner, ToastCorner::TopRight);
    assert_eq!(config.toast.margin_x, 24);
    assert_eq!(config.toast.margin_y, 24);
    assert_eq!(config.toast.font_size, 13);
    assert_eq!(config.toast.corner_radius, 4.0);
    assert!(config.toast.show_sector_direction);

    assert!(config.startup_toast.enabled);
    assert_eq!(config.startup_toast.duration_ms, 5000);
    assert_eq!(config.startup_toast.corner, ToastCorner::TopRight);
    assert_eq!(config.startup_toast.text, "WinPie is active (Press Win+Esc)");

    assert!(config.hint_toast.enabled);
    assert_eq!(config.hint_toast.corner, ToastCorner::BottomRight);
    assert_eq!(config.hint_toast.font_size, 11);

    // 7. Universal Theme
    assert_eq!(config.theme.accent_color, "#00AFFF");
    assert_eq!(config.theme.accent_opacity, 1.0);
    assert_eq!(config.theme.main_bg_color, "#14161C");
    assert_eq!(config.theme.main_bg_opacity, 0.94);
    assert_eq!(config.theme.secondary_bg_color, "#1E222B");
    assert_eq!(config.theme.secondary_bg_opacity, 0.80);
    assert_eq!(config.theme.border_color, "#3C465A");
    assert_eq!(config.theme.border_opacity, 0.85);
    assert_eq!(config.theme.text_primary, "#FFFFFF");
    assert_eq!(config.theme.text_secondary, "#A0A5B5");
    assert_eq!(config.theme.text_accent, "#00AFFF");

    assert_eq!(config.theme.wheel.spoke_color, "#EBEBEB");
    assert_eq!(config.theme.wheel.rim_color, "#F0F0F0");
    assert_eq!(config.theme.wheel.hover_glow_color, "#00AFFF");
    assert_eq!(config.theme.wheel.sector_bg_color, "#16181E");
    assert_eq!(config.theme.wheel.deadzone_bg_color, "#141414");
    assert_eq!(config.theme.wheel.deadzone_border_color, "#F0F0F0");

    // 8. Contextual Profiles
    assert_eq!(config.profiles.len(), 2);
    let vscode_prof = &config.profiles[0];
    assert_eq!(vscode_prof.name, "Visual Studio Code");
    assert_eq!(vscode_prof.match_rules.process.as_deref(), Some("code.exe"));
    assert_eq!(
        config.get_label_for_sector_with_profile(Sector::N, Some(0)),
        "Format Document"
    );

    let explorer_prof = &config.profiles[1];
    assert_eq!(explorer_prof.name, "Windows File Explorer");
    assert_eq!(explorer_prof.match_rules.process.as_deref(), Some("explorer.exe"));
    assert_eq!(explorer_prof.match_rules.window_class.as_deref(), Some("CabinetWClass"));
    assert_eq!(
        config.get_label_for_sector_with_profile(Sector::N, Some(1)),
        "New Folder"
    );
}

#[test]
fn test_config_discovery_priority() {
    assert!(discover_config_path().is_some());
    let (config, path) = AppConfig::discover_and_load();
    assert!(path.is_some(), "Config path must be discovered in CWD");
    let p = path.unwrap();
    assert!(
        p.ends_with("config.yaml") || p.ends_with("config.example.yaml"),
        "Discovered path should be config.yaml or config.example.yaml, got {:?}",
        p
    );
    assert_eq!(config.activation.modifier, "win");
}

#[test]
fn test_config_hot_reload_simulation() {
    let temp_dir = std::env::temp_dir().join(format!("winpie_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let test_file = temp_dir.join("test_config.yaml");

    // Initial content
    let initial_yaml = r#"
activation:
  modifier: win
wheel:
  labels:
    N: "Initial State"
"#;
    fs::write(&test_file, initial_yaml).expect("Write initial");
    let config = AppConfig::load_or_default(&test_file);
    assert_eq!(config.get_label_for_sector(Sector::N), "Initial State");

    // Simulate edit
    let updated_yaml = r#"
activation:
  modifier: win
wheel:
  labels:
    N: "Hot Reloaded State"
"#;
    fs::write(&test_file, updated_yaml).expect("Write update");
    let reloaded = AppConfig::load_or_default(&test_file);
    assert_eq!(reloaded.get_label_for_sector(Sector::N), "Hot Reloaded State");

    let _ = fs::remove_dir_all(&temp_dir);
}
