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

            // Per spec:
            // WS_POPUP
            // WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED
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
            // Position without activating
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

            // 32-bit ARGB DIB for per-pixel alpha transparency
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

                // Clear entire bitmap to 0 (completely transparent)
                let pixel_count = (self.size * self.size) as usize;
                let pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, pixel_count);
                pixels.fill(0);

                // Draw wheel directly into pixel buffer for perfect anti-aliased alpha transparency
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
        let r_squared = r * r;
        let dz_squared = dz * dz;

        for y in 0..self.size {
            for x in 0..self.size {
                let dx = x as f64 - cx + 0.5;
                let dy = y as f64 - cy + 0.5;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq > r_squared {
                    continue; // Outside outer radius: completely transparent
                }

                let idx = (y * self.size + x) as usize;

                if dist_sq <= dz_squared {
                    // Inside deadzone: subtle dark circle with border
                    let dist = dist_sq.sqrt();
                    if dist >= dz - 1.5 {
                        // Deadzone boundary line (white with alpha 180)
                        // Pre-multiplied ARGB: A=180, R=180, G=180, B=180
                        pixels[idx] = (180 << 24) | (180 << 16) | (180 << 8) | 180;
                    } else {
                        // Deadzone fill: dark translucent
                        // Alpha = 120, R=G=B=20 -> Premultiplied: R=G=B = (20*120)/255 = 9
                        pixels[idx] = (120 << 24) | (9 << 16) | (9 << 8) | 9;
                    }
                } else {
                    // In sector region
                    let angle = crate::geometry::angle_from_north_degrees(dx, dy);
                    let sector = crate::geometry::classify_angle(angle, self.rotation);

                    let is_hovered = hover == Some(sector);

                    // Check outer boundary border
                    let dist = dist_sq.sqrt();
                    if dist >= r - 2.0 {
                        // Outer rim border
                        let alpha = 200u32;
                        let val = (255 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (val << 16) | (val << 8) | val;
                        continue;
                    }

                    // Check sector dividing spokes: lines at (index * 45 + rotation - 22.5) deg
                    let rel_angle = crate::geometry::normalize_degrees(angle - self.rotation + 22.5);
                    let angle_in_slice = rel_angle % 45.0;
                    let is_spoke = angle_in_slice < 0.8 || angle_in_slice > 44.2;

                    if is_spoke {
                        // Spoke separator
                        let alpha = 160u32;
                        let val = (200 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (val << 16) | (val << 8) | val;
                    } else if is_hovered {
                        // Highlighted hovered sector: vibrant cyan/blue accent
                        let alpha = 210u32;
                        let r_col = (0u32 * alpha) / 255;
                        let g_col = (160u32 * alpha) / 255;
                        let b_col = (240u32 * alpha) / 255;
                        pixels[idx] = (alpha << 24) | (r_col << 16) | (g_col << 8) | b_col;
                    } else {
                        // Normal unhovered sector: sleek dark glass
                        let alpha = 150u32;
                        let r_col = (25u32 * alpha) / 255;
                        let g_col = (25u32 * alpha) / 255;
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
        // Spec Section 15 & INV-OVERLAY-003:
        // Must NOT become effective mouse target for pointer movement.
        // Return HTTRANSPARENT on WM_NCHITTEST so OS passes clicks and movement to windows below!
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
