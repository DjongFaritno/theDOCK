#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn run() -> Result<(), String> {
    use std::io::Write;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, SW_SHOW, ShowWindow};

    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid: u32 = args
        .first()
        .ok_or("missing parent PID")?
        .parse()
        .map_err(|_| "invalid parent PID")?;
    let handles: Vec<usize> = args[1..]
        .iter()
        .map(|value| {
            value
                .parse()
                .map_err(|_| "invalid taskbar handle".to_string())
        })
        .collect::<Result<_, _>>()?;
    unsafe {
        let parent = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if parent.is_null() {
            return Err("cannot watch parent process".into());
        }
        println!("READY");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        WaitForSingleObject(parent, u32::MAX);
        for handle in handles {
            let hwnd = handle as HWND;
            let mut name = [0u16; 64];
            let length = GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32);
            let class = String::from_utf16_lossy(&name[..length.max(0) as usize]);
            if class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd" {
                ShowWindow(hwnd, SW_SHOW);
            }
        }
        CloseHandle(parent);
    }
    Ok(())
}
