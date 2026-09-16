use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    DIB_RGB_COLORS, HBRUSH,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::AppConfig;
use crate::geometry::{Point, Sector};
use crate::overlay::render::generate_precomputed_buffers;

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
            let precomputed_buffers = generate_precomputed_buffers(
                config,
                size,
                radius as f64,
                deadzone as f64,
                rotation,
            );

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

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = self.size;
            bmi.bmiHeader.biHeight = -self.size; // Top-down
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
