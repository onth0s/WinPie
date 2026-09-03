use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sector {
    N = 0,
    NE = 1,
    E = 2,
    SE = 3,
    S = 4,
    SW = 5,
    W = 6,
    NW = 7,
}

impl Sector {
    pub const ALL: [Sector; 8] = [
        Sector::N,
        Sector::NE,
        Sector::E,
        Sector::SE,
        Sector::S,
        Sector::SW,
        Sector::W,
        Sector::NW,
    ];

    pub fn from_index(index: usize) -> Self {
        match index % 8 {
            0 => Sector::N,
            1 => Sector::NE,
            2 => Sector::E,
            3 => Sector::SE,
            4 => Sector::S,
            5 => Sector::SW,
            6 => Sector::W,
            7 => Sector::NW,
            _ => unreachable!(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Sector::N => "N",
            Sector::NE => "NE",
            Sector::E => "E",
            Sector::SE => "SE",
            Sector::S => "S",
            Sector::SW => "SW",
            Sector::W => "W",
            Sector::NW => "NW",
        }
    }

    pub fn center_angle_degrees(&self) -> f64 {
        (*self as usize as f64) * 45.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

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
pub enum MouseResolution {
    Commit(Sector),
    Cancel,
    NoOp,
}

/// Normalizes any angle in degrees into [0.0, 360.0).
pub fn normalize_degrees(mut deg: f64) -> f64 {
    deg = deg % 360.0;
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
pub fn evaluate_commit(center: Point, cursor: Point, config: &GeometryConfig) -> MouseResolution {
    let dx = (cursor.x - center.x) as f64;
    let dy = (cursor.y - center.y) as f64;
    let r2 = dx * dx + dy * dy;
    let dz2 = config.deadzone * config.deadzone;
    let r_max2 = config.radius * config.radius;

    if r2 <= dz2 || r2 > r_max2 {
        MouseResolution::Cancel
    } else {
        let angle = angle_from_north_degrees(dx, dy);
        let sector = classify_angle(angle, config.rotation_degrees);
        MouseResolution::Commit(sector)
    }
}
