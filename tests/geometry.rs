use winpie::geometry::{
    angle_from_north_degrees, classify_angle, evaluate_commit, evaluate_hover,
    GeometryConfig, Point, Resolution, Sector,
};

#[test]
fn test_angle_from_north() {
    assert!((angle_from_north_degrees(0.0, -100.0) - 0.0).abs() < 1e-6);
    assert!((angle_from_north_degrees(100.0, 0.0) - 90.0).abs() < 1e-6);
    assert!((angle_from_north_degrees(0.0, 100.0) - 180.0).abs() < 1e-6);
    assert!((angle_from_north_degrees(-100.0, 0.0) - 270.0).abs() < 1e-6);
}

#[test]
fn test_classify_all_eight_sectors_unrotated() {
    let rot = 0.0;
    assert_eq!(classify_angle(0.0, rot), Sector::N);
    assert_eq!(classify_angle(350.0, rot), Sector::N);
    assert_eq!(classify_angle(20.0, rot), Sector::N);
    assert_eq!(classify_angle(22.5, rot), Sector::NE);
    assert_eq!(classify_angle(45.0, rot), Sector::NE);
    assert_eq!(classify_angle(67.5, rot), Sector::E);
    assert_eq!(classify_angle(112.5, rot), Sector::SE);
    assert_eq!(classify_angle(157.5, rot), Sector::S);
    assert_eq!(classify_angle(202.5, rot), Sector::SW);
    assert_eq!(classify_angle(247.5, rot), Sector::W);
    assert_eq!(classify_angle(292.5, rot), Sector::NW);
}

#[test]
fn test_rotation() {
    let rot = 22.5;
    assert_eq!(classify_angle(22.5, rot), Sector::N);
    assert_eq!(classify_angle(0.0, rot), Sector::N);
    assert_eq!(classify_angle(337.4, rot), Sector::NW);
    assert_eq!(classify_angle(45.0, rot), Sector::NE);
}

#[test]
fn test_deadzone_and_hover() {
    let config = GeometryConfig {
        radius: 180.0,
        deadzone: 32.0,
        rotation_degrees: 0.0,
        slices: 8,
    };
    let center = Point::new(500, 500);

    // Center point inside deadzone -> None
    assert_eq!(evaluate_hover(center, center, &config), None);

    // Boundary deadzone (r = 32) -> None
    assert_eq!(evaluate_hover(center, Point::new(500, 468), &config), None);

    // Just outside deadzone (r = 33) -> North
    assert_eq!(evaluate_hover(center, Point::new(500, 467), &config), Some(Sector::N));

    // East inside slice bounds (r = 50) -> East
    assert_eq!(evaluate_hover(center, Point::new(550, 500), &config), Some(Sector::E));

    // Outside outer visual radius (r = 1000 > 180) -> None
    assert_eq!(evaluate_hover(center, Point::new(1500, 500), &config), None);
}

#[test]
fn test_negative_multimonitor_coordinates() {
    let config = GeometryConfig::default();
    let center = Point::new(-1000, -500);

    // Cursor inside deadzone -> Cancel
    assert_eq!(evaluate_hover(center, Point::new(-1000, -500), &config), None);
    assert_eq!(evaluate_commit(center, Point::new(-1000, -500), &config), Resolution::Cancel);

    // Cursor South inside valid slice bounds (dy = +100) -> Commit(S)
    assert_eq!(evaluate_hover(center, Point::new(-1000, -400), &config), Some(Sector::S));
    assert_eq!(evaluate_commit(center, Point::new(-1000, -400), &config), Resolution::Commit(Sector::S));

    // Cursor West inside valid slice bounds (dx = -100) -> Commit(W)
    assert_eq!(evaluate_hover(center, Point::new(-1100, -500), &config), Some(Sector::W));
    assert_eq!(evaluate_commit(center, Point::new(-1100, -500), &config), Resolution::Commit(Sector::W));
}

#[test]
fn test_unified_cancel_bounds() {
    let config = GeometryConfig::default();
    let center = Point::new(0, 0);

    // Left-click in deadzone -> Cancel
    assert_eq!(evaluate_commit(center, Point::new(10, 10), &config), Resolution::Cancel);

    // Left-click outside radius (5000 pixels away) -> Cancel
    assert_eq!(evaluate_commit(center, Point::new(5000, 0), &config), Resolution::Cancel);

    // Left-click in valid sector ring (radius 100, between 32 and 180) -> Commit(E)
    assert_eq!(evaluate_commit(center, Point::new(100, 0), &config), Resolution::Commit(Sector::E));
}

#[test]
fn test_sector_label_config_and_fallback() {
    let yaml_str = r#"
wheel:
  labels:
    N: "Terminal"
    E: "Editor"
"#;
    let config: winpie::config::AppConfig = serde_yaml::from_str(yaml_str).unwrap();

    // Configured labels
    assert_eq!(config.get_label_for_sector(Sector::N), "Terminal");
    assert_eq!(config.get_label_for_sector(Sector::E), "Editor");

    // Unconfigured labels fall back to sector names
    assert_eq!(config.get_label_for_sector(Sector::NE), "NE");
    assert_eq!(config.get_label_for_sector(Sector::S), "S");
    assert_eq!(config.get_label_for_sector(Sector::SW), "SW");
    assert_eq!(config.get_label_for_sector(Sector::W), "W");
    assert_eq!(config.get_label_for_sector(Sector::NW), "NW");
    assert_eq!(config.get_label_for_sector(Sector::SE), "SE");
}

#[test]
fn test_toast_config_deserialization() {
    let yaml_str = r#"
toast:
  enabled: true
  duration_ms: 1500
  corner: top_left
  margin_x: 32
  margin_y: 40
  font_size: 14
  show_sector_direction: false
"#;
    let config: winpie::config::AppConfig = serde_yaml::from_str(yaml_str).unwrap();

    assert!(config.toast.enabled);
    assert_eq!(config.toast.duration_ms, 1500);
    assert_eq!(config.toast.corner, winpie::config::ToastCorner::TopLeft);
    assert_eq!(config.toast.margin_x, 32);
    assert_eq!(config.toast.margin_y, 40);
    assert_eq!(config.toast.font_size, 14);
    assert!(!config.toast.show_sector_direction);
}

#[test]
fn test_toast_default_config() {
    let config = winpie::config::AppConfig::default();
    assert!(config.toast.enabled);
    assert_eq!(config.toast.duration_ms, 1000);
    assert_eq!(config.toast.corner, winpie::config::ToastCorner::TopRight);
    assert_eq!(config.toast.margin_x, 24);
    assert_eq!(config.toast.margin_y, 24);
    assert_eq!(config.toast.corner_radius, 4.0);

    // Startup toast defaults
    assert!(config.startup_toast.enabled);
    assert_eq!(config.startup_toast.duration_ms, 5000);
    assert_eq!(config.startup_toast.corner, winpie::config::ToastCorner::TopRight);
    assert_eq!(config.startup_toast.font_size, 13);
    assert_eq!(config.startup_toast.corner_radius, 4.0);
    assert_eq!(config.startup_toast.text, "WinPie is active (Press Win+Esc)");

    // Hint toast defaults
    assert!(config.hint_toast.enabled);
    assert_eq!(config.hint_toast.corner, winpie::config::ToastCorner::BottomRight);
    assert_eq!(config.hint_toast.font_size, 11);
    assert_eq!(config.hint_toast.corner_radius, 4.0);
}

#[test]
fn test_startup_toast_config_deserialization() {
    let yaml_str = r#"
startup_toast:
  enabled: true
  duration_ms: 7500
  corner: bottom_left
  margin_x: 30
  margin_y: 30
  font_size: 15
  text: "Welcome to WinPie"
"#;
    let config: winpie::config::AppConfig = serde_yaml::from_str(yaml_str).unwrap();
    assert!(config.startup_toast.enabled);
    assert_eq!(config.startup_toast.duration_ms, 7500);
    assert_eq!(config.startup_toast.corner, winpie::config::ToastCorner::BottomLeft);
    assert_eq!(config.startup_toast.margin_x, 30);
    assert_eq!(config.startup_toast.margin_y, 30);
    assert_eq!(config.startup_toast.font_size, 15);
    assert_eq!(config.startup_toast.text, "Welcome to WinPie");
}

#[test]
fn test_theme_config_deserialization() {
    let yaml_str = r##"
theme:
  menu_corner_radius: 6.0
  toast_corner_radius: 6.0
  accent_color: "#FF5500"
  accent_opacity: 0.9
  main_bg_color: "#101010"
  main_bg_opacity: 0.95
  border_color: "#445566"
  border_opacity: 0.8
  text_primary: "#EEEEEE"
  text_secondary: "#888888"
  text_accent: "#FF5500"
  wheel:
    spoke_color: "#CCCCCC"
    spoke_opacity: 0.7
    rim_color: "#FFFFFF"
    rim_opacity: 0.85
    hover_glow_color: "#FF5500"
    hover_glow_opacity: 0.85
    sector_bg_color: "#181818"
    sector_bg_opacity: 0.6
    deadzone_bg_color: "#111111"
    deadzone_bg_opacity: 0.5
    deadzone_border_color: "#FFFFFF"
    deadzone_border_opacity: 0.85
"##;
    let config: winpie::config::AppConfig = serde_yaml::from_str(yaml_str).unwrap();

    assert_eq!(config.theme.menu_corner_radius, 6.0);
    assert_eq!(config.theme.toast_corner_radius, 6.0);
    assert_eq!(config.theme.accent_color, "#FF5500");
    assert_eq!(config.theme.accent_opacity, 0.9);
    assert_eq!(config.theme.main_bg_color, "#101010");
    assert_eq!(config.theme.main_bg_opacity, 0.95);
    assert_eq!(config.theme.border_color, "#445566");
    assert_eq!(config.theme.text_primary, "#EEEEEE");
    assert_eq!(config.theme.text_secondary, "#888888");
    assert_eq!(config.theme.text_accent, "#FF5500");

    assert_eq!(config.theme.wheel.hover_glow_color, "#FF5500");
    assert_eq!(config.theme.wheel.spoke_color, "#CCCCCC");

    // Test parse_hex_color
    let (r, g, b) = winpie::config::parse_hex_color(&config.theme.accent_color);
    assert_eq!((r, g, b), (255.0, 85.0, 0.0));
}

#[test]
fn test_command_config_and_lookup() {
    let yaml_str = r#"
wheel:
  commands:
    E: "sublime.exe"
    N: "wt.exe"
"#;
    let config: winpie::config::AppConfig = serde_yaml::from_str(yaml_str).unwrap();

    assert_eq!(config.get_command_for_sector(Sector::E), Some("sublime.exe"));
    assert_eq!(config.get_command_for_sector(Sector::N), Some("wt.exe"));
    assert_eq!(config.get_command_for_sector(Sector::S), None);
    assert_eq!(config.get_command_for_sector(Sector::W), None);
}

#[test]
fn test_menu_and_tooltip_config_deserialization() {
    let yaml_str = r#"
wheel:
  labels:
    N: "Dev Tools"
  tooltips:
    N: "Dev tools and compilers"
    E: "Sublime text"
  menus:
    N:
      title: "Dev Tools"
      tooltip: "Dev tools submenu tooltip"
      items:
        a:
          label: "VS Code"
          tooltip: "Launch VS Code"
          command: "code.exe"
        d:
          label: "Build Tools"
          menu:
            title: "Build Actions"
            items:
              c:
                label: "Clean"
                command: "cargo clean"
"#;
    let config: winpie::config::AppConfig = serde_yaml::from_str(yaml_str).unwrap();

    assert_eq!(config.get_label_for_sector(Sector::N), "Dev Tools");
    assert_eq!(config.get_tooltip_for_sector(Sector::N), Some("Dev tools and compilers"));
    assert_eq!(config.get_tooltip_for_sector(Sector::E), Some("Sublime text"));
    assert_eq!(config.get_tooltip_for_sector(Sector::S), None);

    let menu_n = config.get_menu_for_sector(Sector::N).expect("Menu N should exist");
    assert_eq!(menu_n.title, "Dev Tools");
    assert_eq!(menu_n.items.len(), 2);

    let item_a = menu_n.items.get(&'a').expect("Item 'a' should exist");
    assert_eq!(item_a.label, "VS Code");
    assert_eq!(item_a.tooltip, Some("Launch VS Code".to_string()));
    assert_eq!(item_a.command, Some("code.exe".to_string()));
    assert!(item_a.menu.is_none());

    let item_d = menu_n.items.get(&'d').expect("Item 'd' should exist");
    assert_eq!(item_d.label, "Build Tools");
    assert!(item_d.command.is_none());
    let nested_d = item_d.menu.as_ref().expect("Nested menu should exist");
    assert_eq!(nested_d.title, "Build Actions");
    let item_c = nested_d.items.get(&'c').expect("Item 'c' should exist");
    assert_eq!(item_c.command, Some("cargo clean".to_string()));
}

#[test]
fn test_execute_command_smoke() {
    // Empty command is a safe no-op
    assert!(winpie::executor::execute_command("").is_ok());
    assert!(winpie::executor::execute_command("   ").is_ok());

    // Valid command spawns without error
    let res = winpie::executor::execute_command("cmd.exe /c exit 0");
    assert!(res.is_ok());
}

#[test]
fn test_point_lparam_roundtrip_multimonitor() {
    let test_cases = [
        Point::new(0, 0),
        Point::new(1920, 1080),
        Point::new(-1920, 0),
        Point::new(0, -1080),
        Point::new(-1920, -1080),
        Point::new(-3840, 2160),
        Point::new(i32::MAX, i32::MIN),
        Point::new(-1, -1),
    ];

    for pt in test_cases {
        let packed = pt.to_lparam();
        let unpacked = Point::from_lparam(packed);
        assert_eq!(pt, unpacked, "Failed roundtrip for point {:?}", pt);
    }
}



