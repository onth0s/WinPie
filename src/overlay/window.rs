use std::f64::consts::PI;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, BLENDFUNCTION, HBRUSH,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::diagnostics::AppConfig;
use crate::geometry::{Point, Sector};

pub struct OverlayWindow {
    hwnd: HWND,
    size: i32,
    radius: i32,
    deadzone: i32,
    rotation: f64,
}

impl OverlayWindow {
    pub fn new(config: &AppConfig, instance: HINSTANCE) -> Result<Self, windows::core::Error> {
        let radius = config.overlay.radius as i32;
        let deadzone = config.overlay.deadzone as i32;
        let rotation = config.wheel.rotation_degrees;
        let size = (radius + 20) * 2; // Pad slightly for drawing borders

        let class_name = windows::core::w!("WinPieOverlayClass");

        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(overlay_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: HICON::default(),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: HBRUSH::default(),
                lpszMenuName: windows::core::PCWSTR::null(),
                lpszClassName: class_name,
                hIconSm: HICON::default(),
            };

            RegisterClassExW(&wc);

            let ex_style = WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED;
            let style = WS_POPUP;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name,
                windows::core::w!("WinPie Overlay"),
                style,
                0,
                0,
                size,
                size,
                None,
                None,
                instance,
                None,
            )?;

            Ok(Self {
                hwnd,
                size,
                radius,
                deadzone,
                rotation,
            })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn show_at(&self, center: Point, hover: Option<Sector>) {
        let half = self.size / 2;
        let left = center.x - half;
        let top = center.y - half;

        self.render(hover);

        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                left,
                top,
                self.size,
                self.size,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn update_hover(&self, hover: Option<Sector>) {
        self.render(hover);
    }

    fn render(&self, hover: Option<Sector>) {
        unsafe {
            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

            let mut bmi: windows::Win32::Graphics::Gdi::BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = self.size;
            bmi.bmiHeader.biHeight = -self.size; // Top-down
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = windows::Win32::Graphics::Gdi::BI_RGB.0;

            let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let bitmap = windows::Win32::Graphics::Gdi::CreateDIBSection(
                mem_dc,
                &bmi,
                windows::Win32::Graphics::Gdi::DIB_RGB_COLORS,
                &mut bits_ptr,
                None,
                0,
            );

            if let Ok(bmp) = bitmap {
                let old_bmp = SelectObject(mem_dc, bmp);

                let pixel_count = (self.size * self.size) as usize;
                let pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, pixel_count);
                pixels.fill(0);

                self.draw_wheel_pixels(pixels, hover);

                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_ALPHA as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };

                let pt_src = POINT { x: 0, y: 0 };
                let sz = SIZE {
                    cx: self.size,
                    cy: self.size,
                };

                let _ = UpdateLayeredWindow(
                    self.hwnd,
                    screen_dc,
                    None,
                    Some(&sz),
                    mem_dc,
                    Some(&pt_src),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                );

                let _ = SelectObject(mem_dc, old_bmp);
                let _ = DeleteObject(bmp);
            }

            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
        }
    }

    fn draw_wheel_pixels(&self, pixels: &mut [u32], hover: Option<Sector>) {
        let cx = self.size as f64 / 2.0;
        let cy = self.size as f64 / 2.0;
        let r = self.radius as f64;
        let dz = self.deadzone as f64;

        // Precompute normal vectors for the 8 spoke dividing rays (at angles k * 45 - 22.5 + rot)
        // Each ray line through origin has unit normal:
        // Ray direction angle from North: phi_k = (k * 45 - 22.5 + rot) degrees
        // dx_ray = sin(phi_k), dy_ray = -cos(phi_k)
        // Ray unit normal (perp): nx = cos(phi_k), ny = sin(phi_k)
        // Distance of point (dx, dy) from ray line is: |dx * nx + dy * ny|
        let mut spoke_normals = [(0.0f64, 0.0f64, 0.0f64, 0.0f64); 8];
        for k in 0..8 {
            let spoke_angle_deg = (k as f64) * 45.0 - 22.5 + self.rotation;
            let rad = spoke_angle_deg * PI / 180.0;
            let dx_ray = rad.sin();
            let dy_ray = -rad.cos();
            // Normal is perpendicular: (cos(rad), sin(rad))
            let nx = rad.cos();
            let ny = rad.sin();
            spoke_normals[k] = (dx_ray, dy_ray, nx, ny);
        }

        let spoke_half_width = 1.0; // Clean, uniform 2px line thickness with anti-aliasing

        for y in 0..self.size {
            for x in 0..self.size {
                let dx = x as f64 - cx + 0.5;
                let dy = y as f64 - cy + 0.5;
                let dist = (dx * dx + dy * dy).sqrt();

                // Anti-aliased outer rim boundary
                if dist > r + 1.0 {
                    continue;
                }

                let idx = (y * self.size + x) as usize;

                if dist <= dz {
                    // Inside deadzone: clean circle with smooth border
                    if dist >= dz - 1.5 {
                        let edge_factor = if dist >= dz - 0.5 {
                            (dz + 0.5 - dist).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        let alpha = (220.0 * edge_factor) as u32;
                        let val = (240 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (val << 16) | (val << 8) | val;
                    } else {
                        // Deadzone interior: subtle dark translucent backdrop with center crosshair dot
                        if dist <= 2.5 {
                            // Center anchor dot
                            let alpha = 230u32;
                            let val = (255 * alpha) / 255;
                            pixels[idx] = (alpha << 24) | (val << 16) | (val << 8) | val;
                        } else {
                            let alpha = 130u32;
                            let val = (20 * alpha) / 255;
                            pixels[idx] = (alpha << 24) | (val << 16) | (val << 8) | val;
                        }
                    }
                } else {
                    // In sector ring (dz < dist <= r)
                    let angle = crate::geometry::angle_from_north_degrees(dx, dy);
                    let sector = crate::geometry::classify_angle(angle, self.rotation);
                    let is_hovered = hover == Some(sector);

                    // Find minimum Euclidean distance to any of the 8 ray lines
                    let mut min_spoke_dist = f64::MAX;
                    for &(dx_ray, dy_ray, nx, ny) in &spoke_normals {
                        // Dot product with ray direction to ensure point is in the forward ray direction
                        let dot = dx * dx_ray + dy * dy_ray;
                        if dot > 0.0 {
                            let perp_dist = (dx * nx + dy * ny).abs();
                            if perp_dist < min_spoke_dist {
                                min_spoke_dist = perp_dist;
                            }
                        }
                    }

                    // Outer border ring
                    let is_outer_rim = dist >= r - 2.0;

                    // Compute smooth alpha for outer circular boundary
                    let outer_alpha_factor = if dist > r - 1.0 {
                        (r + 1.0 - dist).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };

                    if min_spoke_dist <= spoke_half_width + 0.8 {
                        // Anti-aliased constant-width spoke separator
                        let spoke_intensity = (1.0 - (min_spoke_dist - spoke_half_width).max(0.0) / 0.8).clamp(0.0, 1.0);
                        let base_alpha = 190.0 * spoke_intensity * outer_alpha_factor;
                        let alpha = base_alpha as u32;
                        let val = (235 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (val << 16) | (val << 8) | val;
                    } else if is_outer_rim {
                        // Smooth outer rim
                        let alpha = (220.0 * outer_alpha_factor) as u32;
                        let val = (240 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (val << 16) | (val << 8) | val;
                    } else if is_hovered {
                        // Glowing cyan glass highlight for hovered sector
                        let alpha = (220.0 * outer_alpha_factor) as u32;
                        let r_col = (0u32 * alpha) / 255;
                        let g_col = (175u32 * alpha) / 255;
                        let b_col = (255u32 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (r_col << 16) | (g_col << 8) | b_col;
                    } else {
                        // Clean frosted dark glass for unhovered sectors
                        let alpha = (150.0 * outer_alpha_factor) as u32;
                        let r_col = (22u32 * alpha) / 255;
                        let g_col = (24u32 * alpha) / 255;
                        let b_col = (30u32 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (r_col << 16) | (g_col << 8) | b_col;
                    }
                }
            }
        }
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.0.is_null() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
