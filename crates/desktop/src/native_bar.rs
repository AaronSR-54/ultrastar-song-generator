//! Tints the native window chrome so it matches the app palette.
//!
//! Windows 11 exposes the title-bar colours through DWM; on older builds the
//! immersive dark-mode flag is the best available approximation. Other
//! platforms keep their system appearance.

#[cfg(target_os = "windows")]
pub fn tint(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;

    let Ok(handle) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(window) = handle.as_raw() else {
        return;
    };
    let hwnd = window.hwnd.get();

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }

    let colorref = |red: u8, green: u8, blue: u8| -> u32 {
        red as u32 | (green as u32) << 8 | (blue as u32) << 16
    };

    let dark: i32 = 1;
    let caption = colorref(0x0A, 0x0A, 0x0C);
    let text = colorref(0xF4, 0xF4, 0xF7);
    let border = colorref(0x22, 0x23, 0x2A);

    // 20 = DWMWA_USE_IMMERSIVE_DARK_MODE, 19 = the same on older Windows 10.
    // 35/36/34 = caption, text and border colours (Windows 11).
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, 20, &dark as *const i32 as *const c_void, 4);
        let _ = DwmSetWindowAttribute(hwnd, 19, &dark as *const i32 as *const c_void, 4);
        let _ = DwmSetWindowAttribute(hwnd, 35, &caption as *const u32 as *const c_void, 4);
        let _ = DwmSetWindowAttribute(hwnd, 36, &text as *const u32 as *const c_void, 4);
        let _ = DwmSetWindowAttribute(hwnd, 34, &border as *const u32 as *const c_void, 4);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn tint(_frame: &eframe::Frame) {}
