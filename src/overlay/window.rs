use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetMonitorInfoW, MonitorFromPoint, ReleaseDC, AC_SRC_ALPHA, BLENDFUNCTION,
    HBRUSH, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::AppConfig;
use crate::geometry::{Point, Sector};
use crate::overlay::render::generate_precomputed_buffers;
use crate::overlay::surface::DibSurface;

pub struct OverlayWindow {
    hwnd: HWND,
    size: i32,
    surface: DibSurface,
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

            let surface = DibSurface::new(size, size)?;

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
                surface,
                default_buffers,
                profile_buffers,
                active_profile_idx: std::cell::Cell::new(None),
            })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn reload_config(&mut self, config: &AppConfig) {
        let radius = config.overlay.radius as i32;
        let deadzone = config.overlay.deadzone as i32;
        let rotation = config.wheel.rotation_degrees;
        let new_size = (radius + 20) * 2;

        if new_size != self.size {
            if let Ok(new_surface) = DibSurface::new(new_size, new_size) {
                self.surface = new_surface;
                self.size = new_size;

                unsafe {
                    let _ = SetWindowPos(
                        self.hwnd,
                        HWND_TOPMOST,
                        0,
                        0,
                        new_size,
                        new_size,
                        SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
                    );
                }
            }
        }

        self.default_buffers = generate_precomputed_buffers(
            config,
            self.size,
            radius as f64,
            deadzone as f64,
            rotation,
            None,
        );

        let mut profile_buffers = Vec::with_capacity(config.profiles.len());
        for (idx, _) in config.profiles.iter().enumerate() {
            let p_bufs = generate_precomputed_buffers(
                config,
                self.size,
                radius as f64,
                deadzone as f64,
                rotation,
                Some(idx),
            );
            profile_buffers.push(p_bufs);
        }
        self.profile_buffers = profile_buffers;
    }

    pub fn compute_clamped_center(&self, center: Point) -> Point {
        unsafe {
            let pt = POINT { x: center.x, y: center.y };
            let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(hmon, &mut mi);
            let work = mi.rcWork;

            crate::geometry::clamp_center_to_bounds(center, self.size, work.left, work.top, work.right, work.bottom)
        }
    }

    pub fn show_at(&self, center: Point, hover: Option<Sector>, profile_idx: Option<usize>) {
        self.active_profile_idx.set(profile_idx);
        let half = self.size / 2;
        let left = center.x - half;
        let top = center.y - half;

        unsafe {
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
                self.surface.bits,
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
                self.surface.dc,
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
