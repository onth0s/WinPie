#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Packs (x, y) into a 64-bit LPARAM for Win32 thread messaging.
    /// Low 32 bits = x, High 32 bits = y.
    pub fn to_lparam(self) -> isize {
        let low = (self.x as u32) as u64;
        let high = (self.y as u32) as u64;
        ((high << 32) | low) as isize
    }

    /// Unpacks a 64-bit LPARAM back into a signed (x, y) Point,
    /// correctly handling negative multi-monitor coordinates.
    pub fn from_lparam(lparam: isize) -> Self {
        let raw = lparam as u64;
        let x = (raw & 0xFFFF_FFFF) as u32 as i32;
        let y = ((raw >> 32) & 0xFFFF_FFFF) as u32 as i32;
        Self { x, y }
    }
}
