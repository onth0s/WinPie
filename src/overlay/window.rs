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

        // Precompute normal vectors for the 8 spoke dividing rays
        let mut spoke_normals = [(0.0f64, 0.0f64, 0.0f64, 0.0f64); 8];
        for k in 0..8 {
            let spoke_angle_deg = (k as f64) * 45.0 - 22.5 + self.rotation;
            let rad = spoke_angle_deg * PI / 180.0;
            let dx_ray = rad.sin();
            let dy_ray = -rad.cos();
            let nx = rad.cos();
            let ny = rad.sin();
            spoke_normals[k] = (dx_ray, dy_ray, nx, ny);
        }

        // 4x Supersampling offsets (rotated grid for superior anti-aliasing on curves)
        const SAMPLES: [(f64, f64); 4] = [
            (-0.3, -0.1),
            (0.1, -0.3),
            (0.3, 0.1),
            (-0.1, 0.3),
        ];

        for y in 0..self.size {
            for x in 0..self.size {
                let px = x as f64 - cx + 0.5;
                let py = y as f64 - cy + 0.5;

                // Fast distance cull
                let approx_dist = (px * px + py * py).sqrt();
                if approx_dist > r + 2.0 {
                    continue;
                }

                let mut total_a = 0.0f64;
                let mut total_r = 0.0f64;
                let mut total_g = 0.0f64;
                let mut total_b = 0.0f64;

                for &(ox, oy) in &SAMPLES {
                    let sx = px + ox;
                    let sy = py + oy;
                    let dist = (sx * sx + sy * sy).sqrt();

                    if dist > r {
                        // Beyond circumference: completely empty
                        continue;
                    }

                    if dist <= dz {
                        // Inside deadzone
                        if dist >= dz - 1.5 {
                            // Deadzone border ring
                            let a = 220.0;
                            total_a += a;
                            total_r += 240.0 * a / 255.0;
                            total_g += 240.0 * a / 255.0;
                            total_b += 240.0 * a / 255.0;
                        } else if dist <= 2.5 {
                            // Center anchor crosshair dot
                            let a = 230.0;
                            total_a += a;
                            total_r += 255.0 * a / 255.0;
                            total_g += 255.0 * a / 255.0;
                            total_b += 255.0 * a / 255.0;
                        } else {
                            // Dark translucent deadzone core
                            let a = 130.0;
                            total_a += a;
                            total_r += 20.0 * a / 255.0;
                            total_g += 20.0 * a / 255.0;
                            total_b += 20.0 * a / 255.0;
                        }
                    } else {
                        // Sector ring
                        let angle = crate::geometry::angle_from_north_degrees(sx, sy);
                        let sector = crate::geometry::classify_angle(angle, self.rotation);
                        let is_hovered = hover == Some(sector);

                        // Spoke check
                        let mut min_spoke_dist = f64::MAX;
                        for &(dx_ray, dy_ray, nx, ny) in &spoke_normals {
                            let dot = sx * dx_ray + sy * dy_ray;
                            if dot > 0.0 {
                                let perp_dist = (sx * nx + sy * ny).abs();
                                if perp_dist < min_spoke_dist {
                                    min_spoke_dist = perp_dist;
                                }
                            }
                        }

                        let is_spoke = min_spoke_dist <= 1.0;
                        let is_rim = dist >= r - 2.0;

                        if is_spoke {
                            let a = 190.0;
                            total_a += a;
                            total_r += 235.0 * a / 255.0;
                            total_g += 235.0 * a / 255.0;
                            total_b += 235.0 * a / 255.0;
                        } else if is_rim {
                            let a = 220.0;
                            total_a += a;
                            total_r += 240.0 * a / 255.0;
                            total_g += 240.0 * a / 255.0;
                            total_b += 240.0 * a / 255.0;
                        } else if is_hovered {
                            // Cyan glow
                            let a = 220.0;
                            total_a += a;
                            total_r += 0.0 * a / 255.0;
                            total_g += 175.0 * a / 255.0;
                            total_b += 255.0 * a / 255.0;
                        } else {
                            // Frosted glass
                            let a = 150.0;
                            total_a += a;
                            total_r += 22.0 * a / 255.0;
                            total_g += 24.0 * a / 255.0;
                            total_b += 30.0 * a / 255.0;
                        }
                    }
                }

                if total_a > 0.0 {
                    let avg_a = (total_a / 4.0).round() as u32;
                    let avg_r = (total_r / 4.0).round() as u32;
                    let avg_g = (total_g / 4.0).round() as u32;
                    let avg_b = (total_b / 4.0).round() as u32;

                    let idx = (y * self.size + x) as usize;
                    pixels[idx] = (avg_a << 24) | (avg_r << 16) | (avg_g << 8) | avg_b;
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
