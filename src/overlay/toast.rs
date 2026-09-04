use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, FillRect, GetDC, GetMonitorInfoW, MonitorFromPoint, ReleaseDC, SelectObject,
    SetBkMode, SetTextColor, AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CLEARTYPE_QUALITY, DIB_RGB_COLORS, DT_CALCRECT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    HBRUSH, MONITORINFO, MONITOR_DEFAULTTONEAREST, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::diagnostics::{ToastConfig, ToastCorner};
use crate::geometry::Point;

const TOAST_TIMER_ID: usize = 9001;

pub struct ToastOverlay {
    hwnd: HWND,
}

impl ToastOverlay {
    pub fn new(instance: HINSTANCE) -> Result<Self, windows::core::Error> {
        let class_name = windows::core::w!("WinPieToastClass");

        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(toast_wndproc),
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
                windows::core::w!("WinPie Toast"),
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

    pub fn show(&self, anchor: Point, text: &str, config: &ToastConfig) {
        if !config.enabled || text.is_empty() {
            return;
        }

        unsafe {
            // Find active monitor work area (respecting taskbar)
            let pt = POINT { x: anchor.x, y: anchor.y };
            let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(hmon, &mut mi);
            let work_area = mi.rcWork;

            // Measure text dimensions
            let screen_dc = GetDC(None);
            let measure_dc = CreateCompatibleDC(screen_dc);

            let font_height = -config.font_size.abs();
            let family_wide: Vec<u16> = OsStr::new("Segoe UI")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let hfont = CreateFontW(
                font_height,
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

            let old_font = SelectObject(measure_dc, hfont);

            let mut text_wide: Vec<u16> = OsStr::new(text).encode_wide().collect();
            let mut calc_rect = RECT::default();
            DrawTextW(
                measure_dc,
                &mut text_wide,
                &mut calc_rect,
                DT_CALCRECT | DT_NOPREFIX | DT_SINGLELINE,
            );

            let text_w = calc_rect.right - calc_rect.left;
            let text_h = calc_rect.bottom - calc_rect.top;

            // Padding inside toast pill: horizontal 18px on each side + 12px for accent dot, vertical 10px
            let pad_x = 18;
            let pad_y = 10;
            let dot_radius = 4;
            let dot_margin_right = 10;

            let toast_w = text_w + pad_x * 2 + (dot_radius * 2) + dot_margin_right;
            let toast_h = (text_h + pad_y * 2).max(36);

            // Compute corner coordinates inside work_area
            let (x, y) = match config.corner {
                ToastCorner::BottomRight => (
                    work_area.right - toast_w - config.margin_x,
                    work_area.bottom - toast_h - config.margin_y,
                ),
                ToastCorner::BottomLeft => (
                    work_area.left + config.margin_x,
                    work_area.bottom - toast_h - config.margin_y,
                ),
                ToastCorner::TopRight => (
                    work_area.right - toast_w - config.margin_x,
                    work_area.top + config.margin_y,
                ),
                ToastCorner::TopLeft => (
                    work_area.left + config.margin_x,
                    work_area.top + config.margin_y,
                ),
            };

            // Rasterize toast into 32-bit ARGB DIB
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = toast_w;
            bmi.bmiHeader.biHeight = -toast_h; // Top-down
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                measure_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                None,
                0,
            );

            if let Ok(bmp) = bitmap {
                let bmp_old = SelectObject(measure_dc, bmp);

                let pixels = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, (toast_w * toast_h) as usize);

                // 1. Draw rounded card background with border in software
                let corner_r = 10.0;
                let bg_a = 230.0; // ~90% opacity frosted glass
                let bg_r = 20.0;
                let bg_g = 22.0;
                let bg_b = 28.0;

                let border_r = 50.0;
                let border_g = 60.0;
                let border_b = 80.0;

                for py in 0..toast_h {
                    for px in 0..toast_w {
                        let fx = px as f64 + 0.5;
                        let fy = py as f64 + 0.5;

                        // Signed distance to rounded rectangle
                        let dx = (fx - (toast_w as f64 / 2.0)).abs() - ((toast_w as f64 / 2.0) - corner_r);
                        let dy = (fy - (toast_h as f64 / 2.0)).abs() - ((toast_h as f64 / 2.0) - corner_r);

                        let dist = if dx > 0.0 && dy > 0.0 {
                            (dx * dx + dy * dy).sqrt() - corner_r
                        } else {
                            dx.max(dy) - corner_r
                        };

                        let alpha_cov = (0.5 - dist).clamp(0.0, 1.0);

                        if alpha_cov > 0.0 {
                            let is_border = dist >= -1.0;
                            let (c_r, c_g, c_b) = if is_border {
                                (border_r, border_g, border_b)
                            } else {
                                (bg_r, bg_g, bg_b)
                            };

                            let final_a = (bg_a * alpha_cov).round() as u32;
                            let final_r = (c_r * (final_a as f64 / 255.0)).round() as u32;
                            let final_g = (c_g * (final_a as f64 / 255.0)).round() as u32;
                            let final_b = (c_b * (final_a as f64 / 255.0)).round() as u32;

                            let idx = (py * toast_w + px) as usize;
                            pixels[idx] = (final_a << 24) | (final_r << 16) | (final_g << 8) | final_b;
                        } else {
                            let idx = (py * toast_w + px) as usize;
                            pixels[idx] = 0;
                        }
                    }
                }

                // 2. Draw accent blue dot indicator
                let dot_cx = (pad_x + dot_radius) as f64;
                let dot_cy = (toast_h as f64) / 2.0;

                for py in 0..toast_h {
                    for px in 0..(pad_x + dot_radius * 2 + 4) {
                        let dx = (px as f64 + 0.5) - dot_cx;
                        let dy = (py as f64 + 0.5) - dot_cy;
                        let dist = (dx * dx + dy * dy).sqrt() - (dot_radius as f64);
                        let cov = (0.5 - dist).clamp(0.0, 1.0);

                        if cov > 0.0 {
                            let idx = (py * toast_w + px) as usize;
                            let dot_a = cov * 255.0;
                            // Accent cyan/blue: #00AFFF
                            let src_a = dot_a / 255.0;
                            let src_r = 0.0 * src_a;
                            let src_g = 175.0 * src_a;
                            let src_b = 255.0 * src_a;

                            let dst = pixels[idx];
                            let dst_a = ((dst >> 24) & 0xFF) as f64;
                            let dst_r = ((dst >> 16) & 0xFF) as f64;
                            let dst_g = ((dst >> 8) & 0xFF) as f64;
                            let dst_b = (dst & 0xFF) as f64;

                            let out_a = src_a * 255.0 + dst_a * (1.0 - src_a);
                            let out_r = src_r + dst_r * (1.0 - src_a);
                            let out_g = src_g + dst_g * (1.0 - src_a);
                            let out_b = src_b + dst_b * (1.0 - src_a);

                            pixels[idx] = ((out_a.round() as u32) << 24)
                                | ((out_r.round() as u32) << 16)
                                | ((out_g.round() as u32) << 8)
                                | (out_b.round() as u32);
                        }
                    }
                }

                // 3. Render text via ClearType mask offscreen
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
                    let m_old_font = SelectObject(mask_dc, hfont);

                    let black_brush = CreateSolidBrush(COLORREF(0));
                    let full_rc = RECT { left: 0, top: 0, right: toast_w, bottom: toast_h };
                    FillRect(mask_dc, &full_rc, black_brush);
                    let _ = DeleteObject(black_brush);

                    SetBkMode(mask_dc, TRANSPARENT);
                    SetTextColor(mask_dc, COLORREF(0x00FFFFFF));

                    let text_left = pad_x + dot_radius * 2 + dot_margin_right;
                    let mut text_rect = RECT {
                        left: text_left,
                        top: 0,
                        right: text_left + text_w + 4,
                        bottom: toast_h,
                    };

                    DrawTextW(
                        mask_dc,
                        &mut text_wide,
                        &mut text_rect,
                        DT_VCENTER | DT_NOPREFIX | DT_SINGLELINE,
                    );

                    let mask_pixels = std::slice::from_raw_parts(mask_bits as *const u32, (toast_w * toast_h) as usize);

                    for py in 0..toast_h {
                        for px in text_left..(text_left + text_w + 4).min(toast_w) {
                            let idx = (py * toast_w + px) as usize;
                            let val = mask_pixels[idx];
                            let r = (val >> 16) & 0xFF;
                            let g = (val >> 8) & 0xFF;
                            let b = val & 0xFF;
                            let lum = (r * 299 + g * 587 + b * 114) / 1000;

                            if lum > 0 {
                                let src_a = (lum as f64) / 255.0;
                                let src_r = 255.0 * src_a;
                                let src_g = 255.0 * src_a;
                                let src_b = 255.0 * src_a;

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

                    SelectObject(mask_dc, m_old_font);
                    SelectObject(mask_dc, m_old_bmp);
                    let _ = DeleteObject(m_bmp);
                }
                let _ = DeleteDC(mask_dc);

                // Commit to Layered Window
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_ALPHA as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };

                let window_pos = POINT { x, y };
                let window_size = SIZE { cx: toast_w, cy: toast_h };
                let src_point = POINT { x: 0, y: 0 };

                let _ = UpdateLayeredWindow(
                    self.hwnd,
                    screen_dc,
                    Some(&window_pos),
                    Some(&window_size),
                    measure_dc,
                    Some(&src_point),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                );

                // Display window without focus theft
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    toast_w,
                    toast_h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );

                // Set / reset auto-dismiss timer
                let _ = SetTimer(self.hwnd, TOAST_TIMER_ID, config.duration_ms, None);

                SelectObject(measure_dc, bmp_old);
                let _ = DeleteObject(bmp);
            }

            SelectObject(measure_dc, old_font);
            let _ = DeleteObject(hfont);
            let _ = DeleteDC(measure_dc);
            ReleaseDC(None, screen_dc);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = KillTimer(self.hwnd, TOAST_TIMER_ID);
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for ToastOverlay {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.0.is_null() {
                let _ = KillTimer(self.hwnd, TOAST_TIMER_ID);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe extern "system" fn toast_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_TIMER => {
            if wparam.0 == TOAST_TIMER_ID {
                let _ = KillTimer(hwnd, TOAST_TIMER_ID);
                let _ = ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
