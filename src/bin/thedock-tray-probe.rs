//! Read-only probe for Explorer notification-area toolbar availability.

#[cfg(windows)]
fn main() {
    use std::ptr::null;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::Controls::TB_BUTTONCOUNT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, FindWindowW, GetClassNameW, IsWindowVisible, SMTO_ABORTIFHUNG,
        SendMessageTimeoutW,
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe extern "system" fn inspect(hwnd: HWND, _param: LPARAM) -> i32 {
        unsafe {
            let mut name = [0u16; 64];
            let length = GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32);
            let class = String::from_utf16_lossy(&name[..length.max(0) as usize]);
            if class == "ToolbarWindow32" {
                let mut count = 0usize;
                let replied = SendMessageTimeoutW(
                    hwnd,
                    TB_BUTTONCOUNT,
                    0,
                    0,
                    SMTO_ABORTIFHUNG,
                    250,
                    &mut count,
                );
                println!(
                    "ToolbarWindow32 visible={} button_count={}",
                    IsWindowVisible(hwnd) != 0,
                    if replied == 0 {
                        "timeout".into()
                    } else {
                        count.to_string()
                    },
                );
            }
        }
        1
    }

    unsafe {
        for class in ["Shell_TrayWnd", "NotifyIconOverflowWindow"] {
            let hwnd = FindWindowW(wide(class).as_ptr(), null());
            println!("{class} found={}", !hwnd.is_null());
            if !hwnd.is_null() {
                EnumChildWindows(hwnd, Some(inspect), 0);
            }
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Run this probe on Windows 11.");
}
