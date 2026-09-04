use std::f64::consts::PI;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, FillRect, GetDC, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CLEARTYPE_QUALITY,
    DIB_RGB_COLORS, DT_CALCRECT, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    HBRUSH, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::diagnostics::AppConfig;
use crate::geometry::{Point, Sector};

pub struct OverlayWindow {
    hwnd: HWND,
    size: i32,
    precomputed_buffers: [Vec<u32>; 9], // 0 = None (deadzone/unhovered), 1..=8 = Sector::from_index(i-1)
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

            // Precompute all 9 bitmaps (0 = unhovered, 1..=8 = each sector hovered)
            // Pre-rasterizing once on startup means hover switching during interaction is an instantaneous 0ms blit!
            let pixel_count = (size * size) as usize;
            let mut precomputed_buffers: [Vec<u32>; 9] = Default::default();

            for i in 0..9 {
                let hover_opt = if i == 0 {
                    None
                } else {
                    Some(Sector::from_index(i - 1))
                };
                let mut buf = vec![0u32; pixel_count];
                Self::rasterize_wheel_pixels(size, radius as f64, deadzone as f64, rotation, &mut buf, hover_opt);

                if config.rendering.show_labels {
                    Self::overlay_labels(size, radius as f64, deadzone as f64, rotation, config, &mut buf, hover_opt);
                }

                precomputed_buffers[i] = buf;
            }

            Ok(Self {
                hwnd,
                size,
                precomputed_buffers,
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

        self.blit_hover(hover);

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
        self.blit_hover(hover);
    }

    fn blit_hover(&self, hover: Option<Sector>) {
        let buffer_idx = match hover {
            None => 0,
            Some(sector) => (sector as usize) + 1,
        };
        let src_pixels = &self.precomputed_buffers[buffer_idx];

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

                // Instant direct memory copy from precomputed buffer! (sub-microsecond)
                std::ptr::copy_nonoverlapping(
                    src_pixels.as_ptr(),
                    bits_ptr as *mut u32,
                    src_pixels.len(),
                );

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

    fn rasterize_wheel_pixels(
        size: i32,
        r: f64,
        dz: f64,
        rotation: f64,
        pixels: &mut [u32],
        hover: Option<Sector>,
    ) {
        let cx = size as f64 / 2.0;
        let cy = size as f64 / 2.0;

        // Precompute normal vectors for the 8 spoke dividing rays
        let mut spoke_normals = [(0.0f64, 0.0f64, 0.0f64, 0.0f64); 8];
        for k in 0..8 {
            let spoke_angle_deg = (k as f64) * 45.0 - 22.5 + rotation;
            let rad = spoke_angle_deg * PI / 180.0;
            let dx_ray = rad.sin();
            let dy_ray = -rad.cos();
            let nx = rad.cos();
            let ny = rad.sin();
            spoke_normals[k] = (dx_ray, dy_ray, nx, ny);
        }

        // 4x Supersampling offsets (rotated grid)
        const SAMPLES: [(f64, f64); 4] = [
            (-0.3, -0.1),
            (0.1, -0.3),
            (0.3, 0.1),
            (-0.1, 0.3),
        ];

        for y in 0..size {
            for x in 0..size {
                let px = x as f64 - cx + 0.5;
                let py = y as f64 - cy + 0.5;

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
                        continue;
                    }

                    if dist <= dz {
                        // Inside deadzone: smooth circular boundary, NO center dot!
                        if dist >= dz - 1.5 {
                            let a = 220.0;
                            total_a += a;
                            total_r += 240.0 * a / 255.0;
                            total_g += 240.0 * a / 255.0;
                            total_b += 240.0 * a / 255.0;
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
                        let sector = crate::geometry::classify_angle(angle, rotation);
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

                    let idx = (y * size + x) as usize;
                    pixels[idx] = (avg_a << 24) | (avg_r << 16) | (avg_g << 8) | avg_b;
                }
            }
        }
    }

    /// Renders anti-aliased sector labels into the precomputed 32-bit ARGB buffer.
    /// Uses a temporary memory DC and ClearType font, reading the rendered glyph mask
    /// to composite text with clean pre-multiplied alpha onto the radial menu.
    fn overlay_labels(
        size: i32,
        radius: f64,
        deadzone: f64,
        rotation: f64,
        config: &AppConfig,
        pixels: &mut [u32],
        hover: Option<Sector>,
    ) {
        unsafe {
            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = size;
            bmi.bmiHeader.biHeight = -size; // Top-down
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                None,
                0,
            );

            if let Ok(bmp) = bitmap {
                let old_bmp = SelectObject(mem_dc, bmp);

                // Convert font family name to null-terminated UTF-16
                let family_wide: Vec<u16> = OsStr::new(&config.rendering.font.family)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let font_height = -config.rendering.font.size.abs();
                let font_weight = config.rendering.font.weight;

                let hfont = CreateFontW(
                    font_height,
                    0,
                    0,
                    0,
                    font_weight,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    CLEARTYPE_QUALITY.0 as u32,
                    0,
                    windows::core::PCWSTR(family_wide.as_ptr()),
                );

                let old_font = SelectObject(mem_dc, hfont);
                SetBkMode(mem_dc, TRANSPARENT);

                let center = (size as f64) / 2.0;
                let r_ratio = config.rendering.font.radius_ratio.clamp(0.1, 0.95);
                let label_r = deadzone + (radius - deadzone) * r_ratio;

                let text_pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, (size * size) as usize);

                for sector in Sector::ALL {
                    let label = config.get_label_for_sector(sector);
                    if label.is_empty() {
                        continue;
                    }

                    let mut label_wide: Vec<u16> = OsStr::new(label)
                        .encode_wide()
                        .collect();

                    // Calculate radial position for sector center
                    // Sector angle in degrees clockwise from North:
                    let sector_deg = (sector as usize as f64) * 45.0 + rotation;
                    let rad = sector_deg * PI / 180.0;
                    let lx = center + label_r * rad.sin();
                    let ly = center - label_r * rad.cos();

                    // Measure text dimensions
                    let mut calc_rect = RECT::default();
                    DrawTextW(
                        mem_dc,
                        &mut label_wide,
                        &mut calc_rect,
                        DT_CALCRECT | DT_NOPREFIX | DT_SINGLELINE,
                    );

                    let tw = calc_rect.right - calc_rect.left;
                    let th = calc_rect.bottom - calc_rect.top;

                    let x0 = (lx - (tw as f64) / 2.0).round() as i32;
                    let y0 = (ly - (th as f64) / 2.0).round() as i32;
                    let x1 = x0 + tw;
                    let y1 = y0 + th;

                    // Clamp to bitmap boundaries
                    let clip_x0 = x0.max(0).min(size);
                    let clip_y0 = y0.max(0).min(size);
                    let clip_x1 = x1.max(0).min(size);
                    let clip_y1 = y1.max(0).min(size);

                    if clip_x0 >= clip_x1 || clip_y0 >= clip_y1 {
                        continue;
                    }

                    // Clear bounding box in mask DC
                    let clear_rect = RECT {
                        left: clip_x0,
                        top: clip_y0,
                        right: clip_x1,
                        bottom: clip_y1,
                    };
                    let black_brush = CreateSolidBrush(COLORREF(0));
                    FillRect(mem_dc, &clear_rect, black_brush);
                    let _ = DeleteObject(black_brush);

                    // Draw text in pure white to extract ClearType intensity mask
                    SetTextColor(mem_dc, COLORREF(0x00FFFFFF));

                    let mut text_rect = RECT {
                        left: x0,
                        top: y0,
                        right: x1,
                        bottom: y1,
                    };

                    DrawTextW(
                        mem_dc,
                        &mut label_wide,
                        &mut text_rect,
                        DT_CENTER | DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                    );

                    let is_hovered = hover == Some(sector);

                    // Composite text pixels into destination pre-multiplied ARGB buffer
                    for py in clip_y0..clip_y1 {
                        for px in clip_x0..clip_x1 {
                            let idx = (py * size + px) as usize;
                            let mask_val = text_pixels[idx];

                            // Extract grayscale luminance from RGB
                            let r = (mask_val >> 16) & 0xFF;
                            let g = (mask_val >> 8) & 0xFF;
                            let b = mask_val & 0xFF;
                            let lum = (r * 299 + g * 587 + b * 114) / 1000;

                            if lum > 0 {
                                // Compute effective alpha coverage
                                let text_alpha = if is_hovered {
                                    ((lum as f64) * (255.0 / 255.0)).min(255.0)
                                } else {
                                    ((lum as f64) * (205.0 / 255.0)).min(255.0)
                                };

                                let src_a = text_alpha / 255.0;

                                // Target text color: pure bright white for hover, subtle off-white for unhovered
                                let (src_r_val, src_g_val, src_b_val) = if is_hovered {
                                    (255.0, 255.0, 255.0)
                                } else {
                                    (230.0, 235.0, 240.0)
                                };

                                let src_r = src_r_val * src_a;
                                let src_g = src_g_val * src_a;
                                let src_b = src_b_val * src_a;

                                // Destination pixel
                                let dst_pixel = pixels[idx];
                                let dst_a = ((dst_pixel >> 24) & 0xFF) as f64;
                                let dst_r = ((dst_pixel >> 16) & 0xFF) as f64;
                                let dst_g = ((dst_pixel >> 8) & 0xFF) as f64;
                                let dst_b = (dst_pixel & 0xFF) as f64;

                                // Porter-Duff Source-Over compositing with pre-multiplied alpha
                                let out_a = src_a * 255.0 + dst_a * (1.0 - src_a);
                                let out_r = src_r + dst_r * (1.0 - src_a);
                                let out_g = src_g + dst_g * (1.0 - src_a);
                                let out_b = src_b + dst_b * (1.0 - src_a);

                                let final_a = (out_a.round() as u32).min(255);
                                let final_r = (out_r.round() as u32).min(255);
                                let final_g = (out_g.round() as u32).min(255);
                                let final_b = (out_b.round() as u32).min(255);

                                pixels[idx] = (final_a << 24) | (final_r << 16) | (final_g << 8) | final_b;
                            }
                        }
                    }
                }

                SelectObject(mem_dc, old_font);
                let _ = DeleteObject(hfont);
                SelectObject(mem_dc, old_bmp);
                let _ = DeleteObject(bmp);
            }

            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
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
