use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, FillRect, GetDC, GetMonitorInfoW, MonitorFromPoint, ReleaseDC, SelectObject,
    SetBkMode, SetTextColor, AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CLEARTYPE_QUALITY, DIB_RGB_COLORS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    HBRUSH, MONITORINFO, MONITOR_DEFAULTTONEAREST, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{parse_hex_color, MenuDefinition, MenuItem, ThemeConfig};
use crate::geometry::Point;

pub struct ModalMenuOverlay {
    hwnd: HWND,
}

impl ModalMenuOverlay {
    pub fn new(instance: HINSTANCE) -> Result<Self, windows::core::Error> {
        let class_name = windows::core::w!("WinPieModalMenuClass");

        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(modal_menu_wndproc),
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
                windows::core::w!("WinPie Modal Menu"),
                style,
                0,
                0,
                10,
                10,
                None,
                None,
                instance,
                None,
            )?;

            Ok(Self { hwnd })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn show(
        &self,
        anchor: Point,
        nav_stack: &[MenuDefinition],
        highlighted_key: Option<char>,
        theme: &ThemeConfig,
    ) {
        if nav_stack.is_empty() {
            return;
        }

        let current_menu = &nav_stack[nav_stack.len() - 1];

        // Format breadcrumb header
        let breadcrumb = nav_stack
            .iter()
            .map(|m| m.title.as_str())
            .collect::<Vec<_>>()
            .join("  >  ");

        // Collect and sort items by key
        let mut sorted_items: Vec<(char, MenuItem)> = current_menu
            .items
            .iter()
            .map(|(&k, item)| (k, item.clone()))
            .collect();
        sorted_items.sort_by_key(|&(k, _)| k);

        unsafe {
            let pt = POINT { x: anchor.x, y: anchor.y };
            let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(hmon, &mut mi);
            let work_area = mi.rcWork;

            let card_w = 320;
            let header_h = 36;
            let row_h = 32;
            let pad_v = 12;

            let rows_count = sorted_items.len() as i32;
            let card_h = header_h + (rows_count * row_h) + (pad_v * 2);

            // Center card horizontally and vertically around anchor, clamped inside work area
            let mut x = anchor.x - (card_w / 2);
            let mut y = anchor.y - (card_h / 2);

            if x < work_area.left + 16 {
                x = work_area.left + 16;
            } else if x + card_w > work_area.right - 16 {
                x = work_area.right - 16 - card_w;
            }

            if y < work_area.top + 16 {
                y = work_area.top + 16;
            } else if y + card_h > work_area.bottom - 16 {
                y = work_area.bottom - 16 - card_h;
            }

            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = card_w;
            bmi.bmiHeader.biHeight = -card_h; // Top-down
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
                let bmp_old = SelectObject(mem_dc, bmp);
                let pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, (card_w * card_h) as usize);

                // 1. Draw rounded card background with configurable theme corner radius and colors
                let corner_r = theme.menu_corner_radius.max(0.0);
                let bg_a = (theme.main_bg_opacity.clamp(0.0, 1.0) * 255.0).round();
                let (bg_r, bg_g, bg_b) = parse_hex_color(&theme.main_bg_color);

                let border_a = (theme.border_opacity.clamp(0.0, 1.0) * 255.0).round();
                let (border_r, border_g, border_b) = parse_hex_color(&theme.border_color);

                for py in 0..card_h {
                    for px in 0..card_w {
                        let fx = px as f64 + 0.5;
                        let fy = py as f64 + 0.5;

                        let dist = if corner_r > 0.0 {
                            let dx = (fx - (card_w as f64 / 2.0)).abs() - ((card_w as f64 / 2.0) - corner_r);
                            let dy = (fy - (card_h as f64 / 2.0)).abs() - ((card_h as f64 / 2.0) - corner_r);

                            if dx > 0.0 && dy > 0.0 {
                                (dx * dx + dy * dy).sqrt() - corner_r
                            } else {
                                dx.max(dy) - corner_r
                            }
                        } else {
                            let dx = (fx - (card_w as f64 / 2.0)).abs() - (card_w as f64 / 2.0);
                            let dy = (fy - (card_h as f64 / 2.0)).abs() - (card_h as f64 / 2.0);
                            dx.max(dy)
                        };

                        let alpha_cov = (0.5 - dist).clamp(0.0, 1.0);

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

                            let idx = (py * card_w + px) as usize;
                            pixels[idx] = (final_a << 24) | (final_r << 16) | (final_g << 8) | final_b;
                        } else {
                            let idx = (py * card_w + px) as usize;
                            pixels[idx] = 0;
                        }
                    }
                }

                // 2. Draw active / hovered row highlight
                let (accent_r, accent_g, accent_b) = parse_hex_color(&theme.accent_color);
                let glow_a = (theme.accent_opacity.clamp(0.0, 1.0) * 0.25).clamp(0.0, 1.0);

                for (row_idx, &(k, _)) in sorted_items.iter().enumerate() {
                    let is_highlighted = highlighted_key == Some(k);
                    if is_highlighted {
                        let row_top = pad_v + header_h + (row_idx as i32 * row_h);
                        let row_bottom = row_top + row_h - 2;
                        let row_left = 10;
                        let row_right = card_w - 10;

                        for py in row_top..row_bottom {
                            for px in row_left..row_right {
                                let idx = (py * card_w + px) as usize;
                                let glow_r_val = accent_r * glow_a;
                                let glow_g_val = accent_g * glow_a;
                                let glow_b_val = accent_b * glow_a;

                                let dst = pixels[idx];
                                let dst_a = ((dst >> 24) & 0xFF) as f64;
                                let dst_r = ((dst >> 16) & 0xFF) as f64;
                                let dst_g = ((dst >> 8) & 0xFF) as f64;
                                let dst_b = (dst & 0xFF) as f64;

                                let out_a = glow_a * 255.0 + dst_a * (1.0 - glow_a);
                                let out_r = glow_r_val + dst_r * (1.0 - glow_a);
                                let out_g = glow_g_val + dst_g * (1.0 - glow_a);
                                let out_b = glow_b_val + dst_b * (1.0 - glow_a);

                                pixels[idx] = ((out_a.round() as u32).min(255) << 24)
                                    | ((out_r.round() as u32).min(255) << 16)
                                    | ((out_g.round() as u32).min(255) << 8)
                                    | ((out_b.round() as u32).min(255));
                            }
                        }
                    }
                }

                // 3. Render typography via GDI mask offscreen with theme colors
                let mask_dc = CreateCompatibleDC(screen_dc);
                let mut mask_bits: *mut std::ffi::c_void = std::ptr::null_mut();
                let mask_bmp = CreateDIBSection(
                    mask_dc,
                    &bmi,
                    DIB_RGB_COLORS,
                    &mut mask_bits,
                    None,
                    0,
                );

                if let Ok(m_bmp) = mask_bmp {
                    let m_old_bmp = SelectObject(mask_dc, m_bmp);

                    let family_wide: Vec<u16> = OsStr::new("Segoe UI")
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();

                    let title_font = CreateFontW(
                        -13,
                        0,
                        0,
                        0,
                        700, // Bold
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

                    let item_font = CreateFontW(
                        -13,
                        0,
                        0,
                        0,
                        600, // Semibold
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

                    let badge_font = CreateFontW(
                        -12,
                        0,
                        0,
                        0,
                        700, // Bold
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

                    let black_brush = CreateSolidBrush(COLORREF(0));
                    let full_rc = RECT { left: 0, top: 0, right: card_w, bottom: card_h };
                    FillRect(mask_dc, &full_rc, black_brush);

                    SetBkMode(mask_dc, TRANSPARENT);
                    SetTextColor(mask_dc, COLORREF(0x00FFFFFF));

                    let mask_pixels = std::slice::from_raw_parts(mask_bits as *const u32, (card_w * card_h) as usize);

                    let color_primary = parse_hex_color(&theme.text_primary);
                    let color_accent = parse_hex_color(&theme.text_accent);
                    let color_secondary = parse_hex_color(&theme.text_secondary);

                    // 3.1 Draw Breadcrumb Title
                    let old_f = SelectObject(mask_dc, title_font);
                    let mut title_wide: Vec<u16> = OsStr::new(&breadcrumb).encode_wide().collect();
                    let mut title_rc = RECT {
                        left: 18,
                        top: pad_v,
                        right: card_w - 18,
                        bottom: pad_v + header_h,
                    };
                    DrawTextW(
                        mask_dc,
                        &mut title_wide,
                        &mut title_rc,
                        DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                    );
                    composite_text_rect(mask_pixels, pixels, card_w, card_h, title_rc, color_primary, 1.0);

                    // 3.2 Draw Item Rows
                    for (row_idx, &(k, ref item)) in sorted_items.iter().enumerate() {
                        let row_y = pad_v + header_h + (row_idx as i32 * row_h);
                        let is_branch = item.menu.is_some();

                        // Clear row area in mask
                        let row_rc = RECT {
                            left: 0,
                            top: row_y,
                            right: card_w,
                            bottom: row_y + row_h,
                        };
                        FillRect(mask_dc, &row_rc, black_brush);

                        // Draw Badge "[ A ]"
                        let badge_str = format!("[ {} ]", k.to_ascii_uppercase());
                        let mut badge_wide: Vec<u16> = OsStr::new(&badge_str).encode_wide().collect();
                        SelectObject(mask_dc, badge_font);
                        let mut badge_rc = RECT {
                            left: 18,
                            top: row_y,
                            right: 64,
                            bottom: row_y + row_h,
                        };
                        DrawTextW(
                            mask_dc,
                            &mut badge_wide,
                            &mut badge_rc,
                            DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                        );
                        composite_text_rect(mask_pixels, pixels, card_w, card_h, badge_rc, color_accent, 1.0);

                        // Draw Label
                        let mut label_wide: Vec<u16> = OsStr::new(&item.label).encode_wide().collect();
                        SelectObject(mask_dc, item_font);
                        let mut label_rc = RECT {
                            left: 68,
                            top: row_y,
                            right: card_w - 36,
                            bottom: row_y + row_h,
                        };
                        DrawTextW(
                            mask_dc,
                            &mut label_wide,
                            &mut label_rc,
                            DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                        );
                        composite_text_rect(mask_pixels, pixels, card_w, card_h, label_rc, color_primary, 1.0);

                        // Draw Branch Arrow if child submenu
                        if is_branch {
                            let mut arrow_wide: Vec<u16> = OsStr::new(">").encode_wide().collect();
                            let mut arrow_rc = RECT {
                                left: card_w - 30,
                                top: row_y,
                                right: card_w - 16,
                                bottom: row_y + row_h,
                            };
                            DrawTextW(
                                mask_dc,
                                &mut arrow_wide,
                                &mut arrow_rc,
                                DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                            );
                            composite_text_rect(mask_pixels, pixels, card_w, card_h, arrow_rc, color_secondary, 0.85);
                        }
                    }

                    let _ = DeleteObject(black_brush);
                    SelectObject(mask_dc, old_f);
                    let _ = DeleteObject(title_font);
                    let _ = DeleteObject(item_font);
                    let _ = DeleteObject(badge_font);
                    SelectObject(mask_dc, m_old_bmp);
                    let _ = DeleteObject(m_bmp);
                }
                let _ = DeleteDC(mask_dc);

                // Commit Layered Window update
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_ALPHA as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };

                let window_pos = POINT { x, y };
                let window_size = SIZE { cx: card_w, cy: card_h };
                let src_point = POINT { x: 0, y: 0 };

                let _ = UpdateLayeredWindow(
                    self.hwnd,
                    screen_dc,
                    Some(&window_pos),
                    Some(&window_size),
                    mem_dc,
                    Some(&src_point),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                );

                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    card_w,
                    card_h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );

                SelectObject(mem_dc, bmp_old);
                let _ = DeleteObject(bmp);
            }

            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
        }
    }

    /// Performs hit-testing to check if the cursor is hovering over a modal menu item row.
    pub fn hit_test(
        &self,
        cursor: Point,
        anchor: Point,
        nav_stack: &[MenuDefinition],
    ) -> Option<(char, MenuItem)> {
        if nav_stack.is_empty() {
            return None;
        }

        let current_menu = &nav_stack[nav_stack.len() - 1];
        let mut sorted_items: Vec<(char, MenuItem)> = current_menu
            .items
            .iter()
            .map(|(&k, item)| (k, item.clone()))
            .collect();
        sorted_items.sort_by_key(|&(k, _)| k);

        let rows_count = sorted_items.len() as i32;
        if rows_count == 0 {
            return None;
        }

        let card_w = 320;
        let header_h = 36;
        let row_h = 32;
        let pad_v = 12;
        let card_h = header_h + (rows_count * row_h) + (pad_v * 2);

        unsafe {
            let pt = POINT { x: anchor.x, y: anchor.y };
            let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(hmon, &mut mi);
            let work_area = mi.rcWork;

            let mut x = anchor.x - (card_w / 2);
            let mut y = anchor.y - (card_h / 2);

            if x < work_area.left + 16 {
                x = work_area.left + 16;
            } else if x + card_w > work_area.right - 16 {
                x = work_area.right - 16 - card_w;
            }

            if y < work_area.top + 16 {
                y = work_area.top + 16;
            } else if y + card_h > work_area.bottom - 16 {
                y = work_area.bottom - 16 - card_h;
            }

            if cursor.x >= x && cursor.x < x + card_w && cursor.y >= y && cursor.y < y + card_h {
                let rel_y = cursor.y - y;
                let rows_top = pad_v + header_h;
                if rel_y >= rows_top {
                    let row_idx = (rel_y - rows_top) / row_h;
                    if row_idx >= 0 && (row_idx as usize) < sorted_items.len() {
                        let (k, ref item) = sorted_items[row_idx as usize];
                        return Some((k, item.clone()));
                    }
                }
            }
        }

        None
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

fn composite_text_rect(
    mask_pixels: &[u32],
    pixels: &mut [u32],
    card_w: i32,
    card_h: i32,
    rect: RECT,
    text_color: (f64, f64, f64),
    text_opacity: f64,
) {
    let top = rect.top.max(0).min(card_h);
    let bottom = rect.bottom.max(0).min(card_h);
    let left = rect.left.max(0).min(card_w);
    let right = rect.right.max(0).min(card_w);

    for py in top..bottom {
        for px in left..right {
            let idx = (py * card_w + px) as usize;
            let val = mask_pixels[idx];
            let r = (val >> 16) & 0xFF;
            let g = (val >> 8) & 0xFF;
            let b = val & 0xFF;
            let lum = (r * 299 + g * 587 + b * 114) / 1000;

            if lum > 0 {
                let src_a = ((lum as f64) / 255.0) * text_opacity;
                let src_r = text_color.0 * src_a;
                let src_g = text_color.1 * src_a;
                let src_b = text_color.2 * src_a;

                let dst = pixels[idx];
                let dst_a = ((dst >> 24) & 0xFF) as f64;
                let dst_r = ((dst >> 16) & 0xFF) as f64;
                let dst_g = ((dst >> 8) & 0xFF) as f64;
                let dst_b = (dst & 0xFF) as f64;

                let out_a = src_a * 255.0 + dst_a * (1.0 - src_a);
                let out_r = src_r + dst_r * (1.0 - src_a);
                let out_g = src_g + dst_g * (1.0 - src_a);
                let out_b = src_b + dst_b * (1.0 - src_a);

                pixels[idx] = ((out_a.round() as u32).min(255) << 24)
                    | ((out_r.round() as u32).min(255) << 16)
                    | ((out_g.round() as u32).min(255) << 8)
                    | ((out_b.round() as u32).min(255));
            }
        }
    }
}

impl Drop for ModalMenuOverlay {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.0.is_null() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe extern "system" fn modal_menu_wndproc(
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
