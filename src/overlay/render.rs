use std::f64::consts::PI;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, FillRect, GetDC, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLEARTYPE_QUALITY, DIB_RGB_COLORS,
    DT_CALCRECT, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, TRANSPARENT,
};

use crate::config::{parse_hex_color, AppConfig, ThemeConfig};
use crate::geometry::Sector;

/// Generates all 9 precomputed 32-bit ARGB buffers:
/// Buffer index 0 = unhovered (deadzone / none)
/// Buffer indices 1..=8 = Sector::from_index(i-1) hovered
pub fn generate_precomputed_buffers(
    config: &AppConfig,
    size: i32,
    radius: f64,
    deadzone: f64,
    rotation: f64,
    profile_idx: Option<usize>,
) -> [Vec<u32>; 9] {
    let pixel_count = (size * size) as usize;
    let mut precomputed_buffers: [Vec<u32>; 9] = Default::default();
    let theme = config.get_theme_for_profile(profile_idx);

    for (i, buf) in precomputed_buffers.iter_mut().enumerate() {
        let hover_opt = if i == 0 {
            None
        } else {
            Some(Sector::from_index(i - 1))
        };
        buf.resize(pixel_count, 0u32);
        rasterize_wheel_pixels(size, radius, deadzone, rotation, theme, buf, hover_opt);

        if config.rendering.show_labels {
            overlay_labels(size, radius, deadzone, rotation, config, theme, profile_idx, buf, hover_opt);
        }
    }

    precomputed_buffers
}

pub fn rasterize_wheel_pixels(
    size: i32,
    r: f64,
    dz: f64,
    rotation: f64,
    theme: &ThemeConfig,
    pixels: &mut [u32],
    hover: Option<Sector>,
) {
    let cx = size as f64 / 2.0;
    let cy = size as f64 / 2.0;

    // Theme color parsing
    let (spoke_r, spoke_g, spoke_b) = parse_hex_color(&theme.wheel.spoke_color);
    let spoke_a = (theme.wheel.spoke_opacity.clamp(0.0, 1.0) * 255.0).round();

    let (rim_r, rim_g, rim_b) = parse_hex_color(&theme.wheel.rim_color);
    let rim_a = (theme.wheel.rim_opacity.clamp(0.0, 1.0) * 255.0).round();

    let (hover_r, hover_g, hover_b) = parse_hex_color(&theme.wheel.hover_glow_color);
    let hover_a = (theme.wheel.hover_glow_opacity.clamp(0.0, 1.0) * 255.0).round();

    let (sec_r, sec_g, sec_b) = parse_hex_color(&theme.wheel.sector_bg_color);
    let sec_a = (theme.wheel.sector_bg_opacity.clamp(0.0, 1.0) * 255.0).round();

    let (dz_r, dz_g, dz_b) = parse_hex_color(&theme.wheel.deadzone_bg_color);
    let dz_a = (theme.wheel.deadzone_bg_opacity.clamp(0.0, 1.0) * 255.0).round();

    let (dz_border_r, dz_border_g, dz_border_b) = parse_hex_color(&theme.wheel.deadzone_border_color);
    let dz_border_a = (theme.wheel.deadzone_border_opacity.clamp(0.0, 1.0) * 255.0).round();

    // Precompute normal vectors for the 8 spoke dividing rays
    let mut spoke_normals = [(0.0f64, 0.0f64, 0.0f64, 0.0f64); 8];
    for (k, normal) in spoke_normals.iter_mut().enumerate() {
        let spoke_angle_deg = (k as f64) * 45.0 - 22.5 + rotation;
        let rad = spoke_angle_deg * PI / 180.0;
        let dx_ray = rad.sin();
        let dy_ray = -rad.cos();
        let nx = rad.cos();
        let ny = rad.sin();
        *normal = (dx_ray, dy_ray, nx, ny);
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
                    // Inside deadzone: smooth circular boundary, NO center dot
                    if dist >= dz - 1.5 {
                        let a = dz_border_a;
                        total_a += a;
                        total_r += dz_border_r * a / 255.0;
                        total_g += dz_border_g * a / 255.0;
                        total_b += dz_border_b * a / 255.0;
                    } else {
                        // Dark translucent deadzone core
                        let a = dz_a;
                        total_a += a;
                        total_r += dz_r * a / 255.0;
                        total_g += dz_g * a / 255.0;
                        total_b += dz_b * a / 255.0;
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
                        let a = spoke_a;
                        total_a += a;
                        total_r += spoke_r * a / 255.0;
                        total_g += spoke_g * a / 255.0;
                        total_b += spoke_b * a / 255.0;
                    } else if is_rim {
                        let a = rim_a;
                        total_a += a;
                        total_r += rim_r * a / 255.0;
                        total_g += rim_g * a / 255.0;
                        total_b += rim_b * a / 255.0;
                    } else if is_hovered {
                        let a = hover_a;
                        total_a += a;
                        total_r += hover_r * a / 255.0;
                        total_g += hover_g * a / 255.0;
                        total_b += hover_b * a / 255.0;
                    } else {
                        let a = sec_a;
                        total_a += a;
                        total_r += sec_r * a / 255.0;
                        total_g += sec_g * a / 255.0;
                        total_b += sec_b * a / 255.0;
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
#[allow(clippy::too_many_arguments)]
pub fn overlay_labels(
    size: i32,
    radius: f64,
    deadzone: f64,
    rotation: f64,
    config: &AppConfig,
    theme: &ThemeConfig,
    profile_idx: Option<usize>,
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
                let label = config.get_label_for_sector_with_profile(sector, profile_idx);
                if label.is_empty() {
                    continue;
                }

                let mut label_wide: Vec<u16> = OsStr::new(label)
                    .encode_wide()
                    .collect();

                // Calculate radial position for sector center:
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
                let (hover_txt_r, hover_txt_g, hover_txt_b) = parse_hex_color(&theme.text_primary);
                let (unhover_txt_r, unhover_txt_g, unhover_txt_b) = parse_hex_color(&theme.text_secondary);

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
                                (lum as f64).min(255.0)
                            } else {
                                ((lum as f64) * (205.0 / 255.0)).min(255.0)
                            };

                            let src_a = text_alpha / 255.0;

                            // Target text color from theme
                            let (src_r_val, src_g_val, src_b_val) = if is_hovered {
                                (hover_txt_r, hover_txt_g, hover_txt_b)
                            } else {
                                (unhover_txt_r, unhover_txt_g, unhover_txt_b)
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
