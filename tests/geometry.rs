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
    let config: winpie::diagnostics::AppConfig = serde_yaml::from_str(yaml_str).unwrap();

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
    let config: winpie::diagnostics::AppConfig = serde_yaml::from_str(yaml_str).unwrap();

    assert!(config.toast.enabled);
    assert_eq!(config.toast.duration_ms, 1500);
    assert_eq!(config.toast.corner, winpie::diagnostics::ToastCorner::TopLeft);
    assert_eq!(config.toast.margin_x, 32);
    assert_eq!(config.toast.margin_y, 40);
    assert_eq!(config.toast.font_size, 14);
    assert!(!config.toast.show_sector_direction);
}

#[test]
fn test_toast_default_config() {
    let config = winpie::diagnostics::AppConfig::default();
    assert!(config.toast.enabled);
    assert_eq!(config.toast.duration_ms, 1000);
    assert_eq!(config.toast.corner, winpie::diagnostics::ToastCorner::BottomRight);
    assert_eq!(config.toast.margin_x, 24);
    assert_eq!(config.toast.margin_y, 24);
}

