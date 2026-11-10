use std::f64::consts::PI;

pub mod point;
pub mod sector;

pub use point::Point;
pub use sector::Sector;

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryConfig {
    pub radius: f64,
    pub deadzone: f64,
    pub rotation_degrees: f64,
    pub slices: usize,
}

impl Default for GeometryConfig {
    fn default() -> Self {
        Self {
            radius: 180.0,
            deadzone: 32.0,
            rotation_degrees: 0.0,
            slices: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Commit(Sector),
    Cancel,
}

/// Normalizes any angle in degrees into [0.0, 360.0).
pub fn normalize_degrees(mut deg: f64) -> f64 {
    deg %= 360.0;
    if deg < 0.0 {
        deg += 360.0;
    }
    deg
}

/// Calculates clockwise angle from North (0 deg) in degrees:
/// dx = x - cx, dy = y - cy
pub fn angle_from_north_degrees(dx: f64, dy: f64) -> f64 {
    let rad = dx.atan2(-dy);
    let deg = rad * 180.0 / PI;
    normalize_degrees(deg)
}

/// Evaluates sector given angle with rotation:
/// θ' = normalize(θ - rotation_degrees)
/// sector_index = floor((θ' + 22.5°) / 45°) mod 8
pub fn classify_angle(theta_deg: f64, rotation_degrees: f64) -> Sector {
    let theta_prime = normalize_degrees(theta_deg - rotation_degrees);
    let shifted = theta_prime + 22.5;
    let idx = ((shifted / 45.0).floor() as usize) % 8;
    Sector::from_index(idx)
}

/// Clamps a desired center point within the given monitor work area bounds
/// such that a window of size `overlay_size` centered at the returned point fits entirely within `[work_left, work_right] x [work_top, work_bottom]`.
pub fn clamp_center_to_bounds(center: Point, overlay_size: i32, work_left: i32, work_top: i32, work_right: i32, work_bottom: i32) -> Point {
    let half = overlay_size / 2;
    let mut left = center.x - half;
    let mut top = center.y - half;

    if left < work_left {
        left = work_left;
    } else if left + overlay_size > work_right {
        left = work_right - overlay_size;
    }

    if top < work_top {
        top = work_top;
    } else if top + overlay_size > work_bottom {
        top = work_bottom - overlay_size;
    }

    Point::new(left + half, top + half)
}

/// Evaluates hover selection.
/// Returns None if inside deadzone (r^2 <= deadzone^2) or outside outer radius (r^2 > radius^2).
/// Returns Some(Sector) if within valid sector ring.
pub fn evaluate_hover(center: Point, cursor: Point, config: &GeometryConfig) -> Option<Sector> {
    let dx = (cursor.x - center.x) as f64;
    let dy = (cursor.y - center.y) as f64;
    let r2 = dx * dx + dy * dy;
    let dz2 = config.deadzone * config.deadzone;
    let r_max2 = config.radius * config.radius;

    if r2 <= dz2 || r2 > r_max2 {
        None
    } else {
        let angle = angle_from_north_degrees(dx, dy);
        Some(classify_angle(angle, config.rotation_degrees))
    }
}

/// Evaluates commit / cancel resolution on left-click:
/// - Inside deadzone (r^2 <= deadzone^2): Cancel (unified cancel condition)
/// - Outside outer radius (r^2 > radius^2): Cancel (unified cancel condition)
/// - Within slice ring (deadzone < r <= radius): Commit(Sector)
pub fn evaluate_commit(center: Point, cursor: Point, config: &GeometryConfig) -> Resolution {
    let dx = (cursor.x - center.x) as f64;
    let dy = (cursor.y - center.y) as f64;
    let r2 = dx * dx + dy * dy;
    let dz2 = config.deadzone * config.deadzone;
    let r_max2 = config.radius * config.radius;

    if r2 <= dz2 || r2 > r_max2 {
        Resolution::Cancel
    } else {
        let angle = angle_from_north_degrees(dx, dy);
        let sector = classify_angle(angle, config.rotation_degrees);
        Resolution::Commit(sector)
    }
}
