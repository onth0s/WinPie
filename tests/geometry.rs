use winpie::geometry::{
    angle_from_north_degrees, classify_angle, evaluate_commit, evaluate_hover,
    GeometryConfig, MouseResolution, Point, Sector,
};

#[test]
fn test_angle_from_north() {
    // North (dx=0, dy=-100) -> 0 deg
    assert!((angle_from_north_degrees(0.0, -100.0) - 0.0).abs() < 1e-6);
    // East (dx=100, dy=0) -> 90 deg
    assert!((angle_from_north_degrees(100.0, 0.0) - 90.0).abs() < 1e-6);
    // South (dx=0, dy=100) -> 180 deg
    assert!((angle_from_north_degrees(0.0, 100.0) - 180.0).abs() < 1e-6);
    // West (dx=-100, dy=0) -> 270 deg
    assert!((angle_from_north_degrees(-100.0, 0.0) - 270.0).abs() < 1e-6);

    // Diagonals
    assert!((angle_from_north_degrees(100.0, -100.0) - 45.0).abs() < 1e-6);
    assert!((angle_from_north_degrees(100.0, 100.0) - 135.0).abs() < 1e-6);
    assert!((angle_from_north_degrees(-100.0, 100.0) - 225.0).abs() < 1e-6);
    assert!((angle_from_north_degrees(-100.0, -100.0) - 315.0).abs() < 1e-6);
}

#[test]
fn test_classify_all_eight_sectors_unrotated() {
    let rot = 0.0;
    // N: [337.5, 22.5)
    assert_eq!(classify_angle(0.0, rot), Sector::N);
    assert_eq!(classify_angle(350.0, rot), Sector::N);
    assert_eq!(classify_angle(20.0, rot), Sector::N);

    // NE: [22.5, 67.5)
    assert_eq!(classify_angle(22.5, rot), Sector::NE);
    assert_eq!(classify_angle(45.0, rot), Sector::NE);
    assert_eq!(classify_angle(67.4, rot), Sector::NE);

    // E: [67.5, 112.5)
    assert_eq!(classify_angle(67.5, rot), Sector::E);
    assert_eq!(classify_angle(90.0, rot), Sector::E);
    assert_eq!(classify_angle(112.4, rot), Sector::E);

    // SE: [112.5, 157.5)
    assert_eq!(classify_angle(112.5, rot), Sector::SE);
    assert_eq!(classify_angle(135.0, rot), Sector::SE);
    assert_eq!(classify_angle(157.4, rot), Sector::SE);

    // S: [157.5, 202.5)
    assert_eq!(classify_angle(157.5, rot), Sector::S);
    assert_eq!(classify_angle(180.0, rot), Sector::S);
    assert_eq!(classify_angle(202.4, rot), Sector::S);

    // SW: [202.5, 247.5)
    assert_eq!(classify_angle(202.5, rot), Sector::SW);
    assert_eq!(classify_angle(225.0, rot), Sector::SW);
    assert_eq!(classify_angle(247.4, rot), Sector::SW);

    // W: [247.5, 292.5)
    assert_eq!(classify_angle(247.5, rot), Sector::W);
    assert_eq!(classify_angle(270.0, rot), Sector::W);
    assert_eq!(classify_angle(292.4, rot), Sector::W);

    // NW: [292.5, 337.5)
    assert_eq!(classify_angle(292.5, rot), Sector::NW);
    assert_eq!(classify_angle(315.0, rot), Sector::NW);
    assert_eq!(classify_angle(337.4, rot), Sector::NW);
}

#[test]
fn test_rotation() {
    // Rotate 22.5 deg clockwise
    let rot = 22.5;
    // With 22.5 deg rotation, angle 45.0 deg becomes 22.5 deg -> Sector::NE (center 45 is now at boundary)
    // Angle 22.5 deg becomes 0 deg -> Sector::N (center of North)
    assert_eq!(classify_angle(22.5, rot), Sector::N);

    // Angle 0 deg becomes -22.5 = 337.5 deg. In N = [337.5, 22.5), 337.5 is exactly the left boundary of N!
    assert_eq!(classify_angle(0.0, rot), Sector::N);

    // Angle 337.4 deg becomes 337.4 - 22.5 = 314.9 deg -> Sector::NW
    assert_eq!(classify_angle(337.4, rot), Sector::NW);

    // Angle 45 deg becomes 22.5 deg -> Sector::NE
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

    // Center point
    assert_eq!(evaluate_hover(center, center, &config), None);

    // Exactly inside deadzone (r = 30 < 32)
    assert_eq!(evaluate_hover(center, Point::new(500, 470), &config), None);

    // Boundary deadzone (r = 32)
    assert_eq!(evaluate_hover(center, Point::new(500, 468), &config), None);

    // Just outside deadzone (r = 33) -> North
    assert_eq!(evaluate_hover(center, Point::new(500, 467), &config), Some(Sector::N));

    // East outside deadzone
    assert_eq!(evaluate_hover(center, Point::new(550, 500), &config), Some(Sector::E));

    // Far outside visual radius (r = 1000) -> Still evaluates Sector
    assert_eq!(evaluate_hover(center, Point::new(1500, 500), &config), Some(Sector::E));
}

#[test]
fn test_negative_multimonitor_coordinates() {
    let config = GeometryConfig::default();
    // Anchor in negative virtual screen space (e.g. secondary monitor at -1920, 0)
    let center = Point::new(-1000, -500);

    // Cursor inside deadzone
    assert_eq!(evaluate_hover(center, Point::new(-1000, -500), &config), None);
    assert_eq!(evaluate_commit(center, Point::new(-1000, -500), &config), MouseResolution::NoOp);

    // Cursor South in negative coordinates (dy = +100)
    assert_eq!(evaluate_hover(center, Point::new(-1000, -400), &config), Some(Sector::S));
    assert_eq!(evaluate_commit(center, Point::new(-1000, -400), &config), MouseResolution::Commit(Sector::S));

    // Cursor West in negative coordinates (dx = -100)
    assert_eq!(evaluate_hover(center, Point::new(-1100, -500), &config), Some(Sector::W));
    assert_eq!(evaluate_commit(center, Point::new(-1100, -500), &config), MouseResolution::Commit(Sector::W));
}

#[test]
fn test_commit_resolution_far_outside_radius() {
    let config = GeometryConfig::default();
    let center = Point::new(0, 0);

    // 5000 pixels away (far outside 180 radius)
    let cursor = Point::new(5000, 0); // East
    assert_eq!(evaluate_commit(center, cursor, &config), MouseResolution::Commit(Sector::E));
}
