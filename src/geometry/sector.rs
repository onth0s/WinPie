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
