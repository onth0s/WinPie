use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    MonitorFromPoint, ReleaseDC, SelectObject, AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
    BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HBRUSH, HDC, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::AppConfig;
use crate::geometry::{Point, Sector};
use crate::overlay::render::generate_precomputed_buffers;

pub struct OverlayWindow {
    hwnd: HWND,
    size: i32,
    mem_dc: HDC,
    bitmap: HBITMAP,
    bits_ptr: *mut u32,
    default_buffers: [Vec<u32>; 9], // 0 = None (deadzone/unhovered), 1..=8 = Sector::from_index(i-1)
    profile_buffers: Vec<[Vec<u32>; 9]>, // Indexed by profile_idx
    active_profile_idx: std::cell::Cell<Option<usize>>,
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

            // Allocate persistent memory DC and DIBSection once at initialization (Zero-allocation hot-path)
            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = size;
            bmi.bmiHeader.biHeight = -size; // Top-down
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB.0;

            let mut bits_raw: *mut std::ffi::c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_raw,
                None,
                0,
            )?;

            let _ = SelectObject(mem_dc, bitmap);
            ReleaseDC(None, screen_dc);
            let bits_ptr = bits_raw as *mut u32;

            // Precompute default buffer set (all 9 states)
            let default_buffers = generate_precomputed_buffers(
                config,
                size,
                radius as f64,
                deadzone as f64,
                rotation,
                None,
            );

            // Precompute profile buffer sets for all configured profiles
            let mut profile_buffers = Vec::with_capacity(config.profiles.len());
            for (idx, _) in config.profiles.iter().enumerate() {
                let p_bufs = generate_precomputed_buffers(
                    config,
                    size,
                    radius as f64,
                    deadzone as f64,
                    rotation,
                    Some(idx),
                );
                profile_buffers.push(p_bufs);
            }

            Ok(Self {
                hwnd,
                size,
                mem_dc,
                bitmap,
                bits_ptr,
                default_buffers,
                profile_buffers,
                active_profile_idx: std::cell::Cell::new(None),
            })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn show_at(&self, center: Point, hover: Option<Sector>, profile_idx: Option<usize>) {
        self.active_profile_idx.set(profile_idx);
        let half = self.size / 2;

        unsafe {
            // Clamp overlay placement within active physical monitor work area (taskbar/edge guard)
            let pt = POINT { x: center.x, y: center.y };
            let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(hmon, &mut mi);
            let work = mi.rcWork;

            let mut left = center.x - half;
            let mut top = center.y - half;

            if left < work.left {
                left = work.left;
            } else if left + self.size > work.right {
                left = work.right - self.size;
            }

            if top < work.top {
                top = work.top;
            } else if top + self.size > work.bottom {
                top = work.bottom - self.size;
            }

            self.blit_hover(hover);

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

        let profile_idx = self.active_profile_idx.get();
        let src_pixels = if let Some(idx) = profile_idx {
            if let Some(p_bufs) = self.profile_buffers.get(idx) {
                &p_bufs[buffer_idx]
            } else {
                &self.default_buffers[buffer_idx]
            }
        } else {
            &self.default_buffers[buffer_idx]
        };

        unsafe {
            let screen_dc = GetDC(None);

            // Zero-allocation, sub-microsecond direct memory copy into persistent DIBSection
            std::ptr::copy_nonoverlapping(
                src_pixels.as_ptr(),
                self.bits_ptr,
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
                self.mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            ReleaseDC(None, screen_dc);
        }
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.bitmap.0.is_null() {
                let _ = DeleteObject(self.bitmap);
            }
            if !self.mem_dc.0.is_null() {
                let _ = DeleteDC(self.mem_dc);
            }
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
