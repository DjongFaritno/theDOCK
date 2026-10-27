#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod win32;

#[cfg(windows)]
fn main() {
    if let Err(error) = win32::run() {
        win32::show_error(&error);
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("theDOCK runs on Windows 11. Build for a Windows target.");
}
