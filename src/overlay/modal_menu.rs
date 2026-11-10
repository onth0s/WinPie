use std::cell::{Cell, RefCell};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, FillRect, GetDC, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CLEARTYPE_QUALITY, DIB_RGB_COLORS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    HBRUSH, HFONT, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{parse_hex_color, MenuDefinition, MenuItem, ThemeConfig};
use crate::geometry::Point;
use super::surface::{composite_text_mask_rect, draw_rounded_card_sdf, query_monitor_work_area, DibSurface};

/// Computes the exact layout boundaries of a modal submenu card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalCardLayout {
    pub card_w: i32,
    pub card_h: i32,
    pub x: i32,
    pub y: i32,
    pub header_h: i32,
    pub row_h: i32,
    pub pad_v: i32,
    pub rows_count: usize,
}

impl ModalCardLayout {
    pub fn compute(anchor: Point, rows_count: usize) -> Self {
        let work_area = query_monitor_work_area(anchor);
        let card_w = 320;
        let header_h = 36;
        let row_h = 32;
        let pad_v = 12;
        let card_h = header_h + (rows_count as i32 * row_h) + (pad_v * 2);

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

        Self {
            card_w,
            card_h,
            x,
            y,
            header_h,
            row_h,
            pad_v,
            rows_count,
        }
    }

    pub fn row_y(&self, row_idx: usize) -> i32 {
        self.pad_v + self.header_h + (row_idx as i32 * self.row_h)
    }

    pub fn hit_test_row(&self, cursor: Point) -> Option<usize> {
        if cursor.x >= self.x && cursor.x < self.x + self.card_w
            && cursor.y >= self.y && cursor.y < self.y + self.card_h
        {
            let rel_y = cursor.y - self.y;
            let rows_top = self.pad_v + self.header_h;
            if rel_y >= rows_top {
                let idx = (rel_y - rows_top) / self.row_h;
                if idx >= 0 && (idx as usize) < self.rows_count {
                    return Some(idx as usize);
                }
            }
        }
        None
    }
}

pub struct ModalMenuOverlay {
    hwnd: HWND,
    title_font: HFONT,
    item_font: HFONT,
    badge_font: HFONT,
    cached_layout: Cell<Option<ModalCardLayout>>,
    cached_items: RefCell<Vec<(char, MenuItem)>>,
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

            Ok(Self {
                hwnd,
                title_font,
                item_font,
                badge_font,
                cached_layout: Cell::new(None),
                cached_items: RefCell::new(Vec::new()),
            })
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

        let layout = ModalCardLayout::compute(anchor, sorted_items.len());
        self.cached_layout.set(Some(layout));
        *self.cached_items.borrow_mut() = sorted_items.clone();

        let mut surface = match DibSurface::new(layout.card_w, layout.card_h) {
            Ok(s) => s,
            Err(_) => return,
        };

        // 1. Draw rounded card background with border via unified SDF
        draw_rounded_card_sdf(
            surface.pixels_mut(),
            layout.card_w,
            layout.card_h,
            theme.menu_corner_radius,
            &theme.main_bg_color,
            theme.main_bg_opacity,
            &theme.border_color,
            theme.border_opacity,
        );

        // 2. Draw active / hovered row highlight glow
        let (accent_r, accent_g, accent_b) = parse_hex_color(&theme.accent_color);
        let glow_a = (theme.accent_opacity.clamp(0.0, 1.0) * 0.25).clamp(0.0, 1.0);
        let pixels = surface.pixels_mut();

        for (row_idx, &(k, _)) in sorted_items.iter().enumerate() {
            if highlighted_key == Some(k) {
                let row_top = layout.row_y(row_idx);
                let row_bottom = row_top + layout.row_h - 2;
                let row_left = 10;
                let row_right = layout.card_w - 10;

                for py in row_top..row_bottom {
                    for px in row_left..row_right {
                        let idx = (py * layout.card_w + px) as usize;
                        let glow_r_val = accent_r * glow_a;
                        let glow_g_val = accent_g * glow_a;
                        let glow_b_val = accent_b * glow_a;

                        pixels[idx] = crate::overlay::surface::blend_premultiplied_over(
                            pixels[idx],
                            glow_r_val,
                            glow_g_val,
                            glow_b_val,
                            glow_a,
                        );
                    }
                }
            }
        }

        // 3. Render typography via offscreen GDI ClearType mask
        unsafe {
            let screen_dc = GetDC(None);
            let mask_dc = CreateCompatibleDC(screen_dc);

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = layout.card_w;
            bmi.bmiHeader.biHeight = -layout.card_h;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut mask_bits: *mut std::ffi::c_void = std::ptr::null_mut();
            if let Ok(m_bmp) = CreateDIBSection(
                mask_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut mask_bits,
                None,
                0,
            ) {
                let m_old_bmp = SelectObject(mask_dc, m_bmp);
                let black_brush = CreateSolidBrush(COLORREF(0));
                let full_rc = RECT { left: 0, top: 0, right: layout.card_w, bottom: layout.card_h };
                FillRect(mask_dc, &full_rc, black_brush);

                SetBkMode(mask_dc, TRANSPARENT);
                SetTextColor(mask_dc, COLORREF(0x00FFFFFF));

                let mask_pixels = std::slice::from_raw_parts(mask_bits as *const u32, (layout.card_w * layout.card_h) as usize);

                // 3.1 Draw Breadcrumb Title
                let old_f = SelectObject(mask_dc, self.title_font);
                let mut title_wide: Vec<u16> = OsStr::new(&breadcrumb).encode_wide().collect();
                let mut title_rc = RECT {
                    left: 18,
                    top: layout.pad_v,
                    right: layout.card_w - 18,
                    bottom: layout.pad_v + layout.header_h,
                };
                DrawTextW(
                    mask_dc,
                    &mut title_wide,
                    &mut title_rc,
                    DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                );
                composite_text_mask_rect(
                    mask_pixels,
                    surface.pixels_mut(),
                    layout.card_w,
                    layout.card_h,
                    title_rc,
                    &theme.text_primary,
                    1.0,
                );

                // 3.2 Draw Item Rows
                for (row_idx, &(k, ref item)) in sorted_items.iter().enumerate() {
                    let row_y = layout.row_y(row_idx);
                    let is_branch = item.menu.is_some();

                    let row_rc = RECT {
                        left: 0,
                        top: row_y,
                        right: layout.card_w,
                        bottom: row_y + layout.row_h,
                    };
                    FillRect(mask_dc, &row_rc, black_brush);

                    // Badge "[ A ]"
                    let badge_str = format!("[ {} ]", k.to_ascii_uppercase());
                    let mut badge_wide: Vec<u16> = OsStr::new(&badge_str).encode_wide().collect();
                    SelectObject(mask_dc, self.badge_font);
                    let mut badge_rc = RECT {
                        left: 18,
                        top: row_y,
                        right: 64,
                        bottom: row_y + layout.row_h,
                    };
                    DrawTextW(
                        mask_dc,
                        &mut badge_wide,
                        &mut badge_rc,
                        DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                    );
                    composite_text_mask_rect(
                        mask_pixels,
                        surface.pixels_mut(),
                        layout.card_w,
                        layout.card_h,
                        badge_rc,
                        &theme.text_accent,
                        1.0,
                    );

                    // Label
                    let mut label_wide: Vec<u16> = OsStr::new(&item.label).encode_wide().collect();
                    SelectObject(mask_dc, self.item_font);
                    let mut label_rc = RECT {
                        left: 68,
                        top: row_y,
                        right: layout.card_w - 36,
                        bottom: row_y + layout.row_h,
                    };
                    DrawTextW(
                        mask_dc,
                        &mut label_wide,
                        &mut label_rc,
                        DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                    );
                    composite_text_mask_rect(
                        mask_pixels,
                        surface.pixels_mut(),
                        layout.card_w,
                        layout.card_h,
                        label_rc,
                        &theme.text_primary,
                        1.0,
                    );

                    // Branch Arrow
                    if is_branch {
                        let mut arrow_wide: Vec<u16> = OsStr::new(">").encode_wide().collect();
                        let mut arrow_rc = RECT {
                            left: layout.card_w - 30,
                            top: row_y,
                            right: layout.card_w - 16,
                            bottom: row_y + layout.row_h,
                        };
                        DrawTextW(
                            mask_dc,
                            &mut arrow_wide,
                            &mut arrow_rc,
                            DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                        );
                        composite_text_mask_rect(
                            mask_pixels,
                            surface.pixels_mut(),
                            layout.card_w,
                            layout.card_h,
                            arrow_rc,
                            &theme.text_secondary,
                            0.85,
                        );
                    }
                }

                let _ = DeleteObject(black_brush);
                SelectObject(mask_dc, old_f);
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

            let window_pos = POINT { x: layout.x, y: layout.y };
            let window_size = SIZE { cx: layout.card_w, cy: layout.card_h };
            let src_point = POINT { x: 0, y: 0 };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                screen_dc,
                Some(&window_pos),
                Some(&window_size),
                surface.dc,
                Some(&src_point),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                layout.x,
                layout.y,
                layout.card_w,
                layout.card_h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );

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

        // Fast path: use cached layout and sorted items if available (0 allocations, 0 Win32 syscalls)
        if let Some(layout) = self.cached_layout.get() {
            let items = self.cached_items.borrow();
            if let Some(row_idx) = layout.hit_test_row(cursor) {
                if row_idx < items.len() {
                    return Some(items[row_idx].clone());
                }
            }
            return None;
        }

        let current_menu = &nav_stack[nav_stack.len() - 1];
        let mut sorted_items: Vec<(char, MenuItem)> = current_menu
            .items
            .iter()
            .map(|(&k, item)| (k, item.clone()))
            .collect();
        sorted_items.sort_by_key(|&(k, _)| k);

        let layout = ModalCardLayout::compute(anchor, sorted_items.len());
        self.cached_layout.set(Some(layout));
        *self.cached_items.borrow_mut() = sorted_items.clone();
        if let Some(row_idx) = layout.hit_test_row(cursor) {
            return Some(sorted_items[row_idx].clone());
        }

        None
    }

    pub fn hide(&self) {
        self.cached_layout.set(None);
        self.cached_items.borrow_mut().clear();
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for ModalMenuOverlay {
    fn drop(&mut self) {
        unsafe {
            if !self.title_font.0.is_null() {
                let _ = DeleteObject(self.title_font);
            }
            if !self.item_font.0.is_null() {
                let _ = DeleteObject(self.item_font);
            }
            if !self.badge_font.0.is_null() {
                let _ = DeleteObject(self.badge_font);
            }
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
