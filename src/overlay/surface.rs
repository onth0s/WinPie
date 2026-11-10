use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    MonitorFromPoint, ReleaseDC, SelectObject, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
    DIB_RGB_COLORS, HBITMAP, HDC, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};

use crate::config::parse_hex_color;
use crate::geometry::Point;

/// Queries the work area bounds of the monitor nearest to `pt` (respecting taskbars).
pub fn query_monitor_work_area(pt: Point) -> RECT {
    unsafe {
        let win_pt = POINT { x: pt.x, y: pt.y };
        let hmon = MonitorFromPoint(win_pt, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(hmon, &mut mi);
        mi.rcWork
    }
}

/// Computes the signed distance from point `(fx, fy)` to a rounded rectangle with half-extents
/// `(half_w, half_h)` and corner radius `corner_r`.
pub fn rounded_rect_sdf(fx: f64, fy: f64, width: i32, height: i32, corner_r: f64) -> f64 {
    let half_w = width as f64 / 2.0;
    let half_h = height as f64 / 2.0;
    let r = corner_r.max(0.0);

    if r > 0.0 {
        let dx = (fx - half_w).abs() - (half_w - r);
        let dy = (fy - half_h).abs() - (half_h - r);
        if dx > 0.0 && dy > 0.0 {
            (dx * dx + dy * dy).sqrt() - r
        } else {
            dx.max(dy) - r
        }
    } else {
        let dx = (fx - half_w).abs() - half_w;
        let dy = (fy - half_h).abs() - half_h;
        dx.max(dy)
    }
}

/// Fills a 32-bit ARGB pixel buffer with a rounded-rectangle card background and border.
#[allow(clippy::too_many_arguments)]
pub fn draw_rounded_card_sdf(
    pixels: &mut [u32],
    width: i32,
    height: i32,
    corner_r: f64,
    bg_color_hex: &str,
    bg_opacity: f64,
    border_color_hex: &str,
    border_opacity: f64,
) {
    let bg_a = (bg_opacity.clamp(0.0, 1.0) * 255.0).round();
    let (bg_r, bg_g, bg_b) = parse_hex_color(bg_color_hex);

    let border_a = (border_opacity.clamp(0.0, 1.0) * 255.0).round();
    let (border_r, border_g, border_b) = parse_hex_color(border_color_hex);

    for py in 0..height {
        for px in 0..width {
            let fx = px as f64 + 0.5;
            let fy = py as f64 + 0.5;

            let dist = rounded_rect_sdf(fx, fy, width, height, corner_r);
            let alpha_cov = (0.5 - dist).clamp(0.0, 1.0);

            let idx = (py * width + px) as usize;
            if alpha_cov > 0.0 {
                let is_border = dist >= -1.0;
                let (c_r, c_g, c_b, c_a) = if is_border {
                    (border_r, border_g, border_b, border_a)
                } else {
                    (bg_r, bg_g, bg_b, bg_a)
                };

                let final_a = (c_a * alpha_cov).round() as u32;
                let final_r = (c_r * (final_a as f64 / 255.0)).round() as u32;
                let final_g = (c_g * (final_a as f64 / 255.0)).round() as u32;
                let final_b = (c_b * (final_a as f64 / 255.0)).round() as u32;

                pixels[idx] = (final_a << 24) | (final_r << 16) | (final_g << 8) | final_b;
            } else {
                pixels[idx] = 0;
            }
        }
    }
}

/// Computes standard Porter-Duff Source-Over compositing with pre-multiplied alpha.
#[inline]
pub fn blend_premultiplied_over(dst: u32, src_r: f64, src_g: f64, src_b: f64, src_a: f64) -> u32 {
    let dst_a = ((dst >> 24) & 0xFF) as f64;
    let dst_r = ((dst >> 16) & 0xFF) as f64;
    let dst_g = ((dst >> 8) & 0xFF) as f64;
    let dst_b = (dst & 0xFF) as f64;

    let out_a = src_a * 255.0 + dst_a * (1.0 - src_a);
    let out_r = src_r + dst_r * (1.0 - src_a);
    let out_g = src_g + dst_g * (1.0 - src_a);
    let out_b = src_b + dst_b * (1.0 - src_a);

    ((out_a.round() as u32).min(255) << 24)
        | ((out_r.round() as u32).min(255) << 16)
        | ((out_g.round() as u32).min(255) << 8)
        | ((out_b.round() as u32).min(255))
}

/// Composites an offscreen ClearType intensity mask into destination pixels using Porter-Duff
/// pre-multiplied alpha source-over blending.
pub fn composite_text_mask_rect(
    mask_pixels: &[u32],
    pixels: &mut [u32],
    surface_w: i32,
    surface_h: i32,
    rect: RECT,
    text_color_hex: &str,
    text_opacity: f64,
) {
    let top = rect.top.max(0).min(surface_h);
    let bottom = rect.bottom.max(0).min(surface_h);
    let left = rect.left.max(0).min(surface_w);
    let right = rect.right.max(0).min(surface_w);

    let (txt_r, txt_g, txt_b) = parse_hex_color(text_color_hex);
    let opacity = text_opacity.clamp(0.0, 1.0);

    for py in top..bottom {
        for px in left..right {
            let idx = (py * surface_w + px) as usize;
            let val = mask_pixels[idx];
            let r = (val >> 16) & 0xFF;
            let g = (val >> 8) & 0xFF;
            let b = val & 0xFF;
            let lum = (r * 299 + g * 587 + b * 114) / 1000;

            if lum > 0 {
                let src_a = ((lum as f64) / 255.0) * opacity;
                let src_r = txt_r * src_a;
                let src_g = txt_g * src_a;
                let src_b = txt_b * src_a;

                pixels[idx] = blend_premultiplied_over(pixels[idx], src_r, src_g, src_b, src_a);
            }
        }
    }
}

/// An offscreen 32-bit ARGB DIBSection with its compatible device context.
pub struct DibSurface {
    pub dc: HDC,
    pub bitmap: HBITMAP,
    pub old_bitmap: HBITMAP,
    pub bits: *mut u32,
    pub width: i32,
    pub height: i32,
}

impl DibSurface {
    pub fn new(width: i32, height: i32) -> Result<Self, windows::core::Error> {
        unsafe {
            let screen_dc = GetDC(None);
            let dc = CreateCompatibleDC(screen_dc);

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height; // Top-down
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut bits_raw: *mut std::ffi::c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_raw,
                None,
                0,
            )?;

            let old_bitmap = HBITMAP(SelectObject(dc, bitmap).0);
            ReleaseDC(None, screen_dc);

            Ok(Self {
                dc,
                bitmap,
                old_bitmap,
                bits: bits_raw as *mut u32,
                width,
                height,
            })
        }
    }

    pub fn pixels_mut(&mut self) -> &mut [u32] {
        unsafe {
            std::slice::from_raw_parts_mut(self.bits, (self.width * self.height) as usize)
        }
    }

    pub fn pixels(&self) -> &[u32] {
        unsafe {
            std::slice::from_raw_parts(self.bits, (self.width * self.height) as usize)
        }
    }
}

impl Drop for DibSurface {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.dc, self.old_bitmap);
            let _ = DeleteObject(self.bitmap);
            let _ = DeleteDC(self.dc);
        }
    }
}
