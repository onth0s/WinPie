use winpie::geometry::{
    angle_from_north_degrees, classify_angle, evaluate_commit, evaluate_hover,
    GeometryConfig, MouseResolution, Point, Sector,
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
    assert_eq!(evaluate_commit(center, Point::new(-1000, -500), &config), MouseResolution::Cancel);

    // Cursor South inside valid slice bounds (dy = +100) -> Commit(S)
    assert_eq!(evaluate_hover(center, Point::new(-1000, -400), &config), Some(Sector::S));
    assert_eq!(evaluate_commit(center, Point::new(-1000, -400), &config), MouseResolution::Commit(Sector::S));

    // Cursor West inside valid slice bounds (dx = -100) -> Commit(W)
    assert_eq!(evaluate_hover(center, Point::new(-1100, -500), &config), Some(Sector::W));
    assert_eq!(evaluate_commit(center, Point::new(-1100, -500), &config), MouseResolution::Commit(Sector::W));
}

#[test]
fn test_unified_cancel_bounds() {
    let config = GeometryConfig::default();
    let center = Point::new(0, 0);

    // Left-click in deadzone -> Cancel
    assert_eq!(evaluate_commit(center, Point::new(10, 10), &config), MouseResolution::Cancel);

    // Left-click outside radius (5000 pixels away) -> Cancel
    assert_eq!(evaluate_commit(center, Point::new(5000, 0), &config), MouseResolution::Cancel);

    // Left-click in valid sector ring (radius 100, between 32 and 180) -> Commit(E)
    assert_eq!(evaluate_commit(center, Point::new(100, 0), &config), MouseResolution::Commit(Sector::E));
}
