//! A small native Win32 shell for the first prototype.
//! Safety: Win32 handles and callback pointers stay on this UI thread.

use std::ffi::c_void;
use std::io::{BufRead, BufReader, Write};
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::ptr::{null, null_mut};
use thedock::{Setting, Settings};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, POINT, RECT, SYSTEMTIME,
};
use windows_sys::Win32::Graphics::Dwm::{
    DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CombineRgn, CreateFontW,
    CreateRoundRectRgn, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_GUI_FONT, DEFAULT_PITCH,
    DeleteObject, EndPaint, FW_MEDIUM, FillRect, GetStockObject, HDC, InvalidateRect,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, RGN_OR, RoundRect, SelectObject, SetBkMode, SetTextColor,
    SetWindowRgn, TRANSPARENT, TextOutW,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows_sys::Win32::System::SystemInformation::{
    GetLocalTime, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcessId, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, MOD_ALT, MOD_CONTROL,
    RegisterHotKey, SendInput, UnregisterHotKey, VK_ESCAPE, VK_LWIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DI_NORMAL, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, DrawIconEx, EnumWindows, GWLP_USERDATA, GetClassNameW,
    GetClientRect, GetCursorPos, GetMessageW, GetSystemMetrics, GetWindowLongPtrW, HICON,
    HWND_TOPMOST, IsWindowVisible, KillTimer, LWA_ALPHA, LoadIconW, MB_ICONERROR, MB_OK,
    MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG, MessageBoxW, PostQuitMessage, RegisterClassW,
    SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SetForegroundWindow,
    SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TPM_RETURNCMD, TrackPopupMenu, TranslateMessage, WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE,
    WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONUP, WM_PAINT, WM_RBUTTONUP, WM_SETTINGCHANGE,
    WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const CLASS_NAME: &str = "theDOCK_prototype";
const DOCK_WIDTH: i32 = 720;
const DOCK_HEIGHT: i32 = 66;
const TRAY_HEIGHT: i32 = 128;
const TIMER_ID: usize = 1;
const MENU_HOTKEY_ID: i32 = 1;
const SETTINGS_MENU: &[(u32, Setting, &str)] = &[
    (8, Setting::Start, "Tombol Start"),
    (1, Setting::Clock, "Jam"),
    (2, Setting::Date, "Tanggal"),
    (3, Setting::Ram, "RAM"),
    (4, Setting::Battery, "Baterai"),
    (5, Setting::Tray, "Tombol Tray"),
    (6, Setting::Glass, "Efek kaca"),
    (7, Setting::HideTaskbar, "Sembunyikan taskbar"),
];

struct App {
    settings: Settings,
    settings_path: PathBuf,
    tray_open: bool,
    icon: HICON,
    taskbar: Option<TaskbarGuard>,
}

struct TaskbarGuard {
    original_visible: Vec<usize>,
    _watchdog: Child,
}

struct InstanceMutex(HANDLE);

impl Drop for InstanceMutex {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

impl TaskbarGuard {
    fn hide() -> Result<Option<Self>, String> {
        let visible = unsafe { visible_taskbars() };
        if visible.is_empty() {
            return Ok(None);
        }

        // The watcher must confirm that it has a live parent handle before the
        // taskbar is hidden. It restores the original visible windows on crash.
        let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command
            .arg("--watchdog")
            .arg(unsafe { GetCurrentProcessId() }.to_string())
            .stdout(Stdio::piped())
            .stdin(Stdio::null());
        for handle in &visible {
            command.arg(handle.to_string());
        }
        let mut watcher = command.spawn().map_err(|e| e.to_string())?;
        let mut ready = String::new();
        if let Some(stdout) = watcher.stdout.take() {
            BufReader::new(stdout)
                .read_line(&mut ready)
                .map_err(|e| e.to_string())?;
        }
        if ready.trim() != "READY" {
            return Err("watchdog taskbar gagal dijalankan; taskbar tetap terlihat".into());
        }
        for handle in &visible {
            unsafe { ShowWindow(*handle as HWND, SW_HIDE) };
        }
        Ok(Some(Self {
            original_visible: visible,
            _watchdog: watcher,
        }))
    }

    fn restore(&self) {
        unsafe { restore_taskbars(&self.original_visible) };
    }
}

impl Drop for TaskbarGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

pub fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--watchdog") {
        return watchdog(&args[2..]);
    }

    let mutex = unsafe { CreateMutexW(null(), 0, wide("Local\\theDOCK-prototype").as_ptr()) };
    if mutex.is_null() {
        return Err("Kunci satu instansi gagal dibuat".into());
    }
    let _instance = InstanceMutex(mutex);
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Err("theDOCK sudah berjalan".into());
    }

    let settings_path = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or("APPDATA tidak tersedia")?
        .join("theDOCK")
        .join("settings.json");
    let settings = Settings::load(&settings_path);

    unsafe {
        let class = wide(CLASS_NAME);
        let title = wide("theDOCK");
        let module = GetModuleHandleW(null());
        if module.is_null() {
            return Err("GetModuleHandleW gagal".into());
        }
        // MAKEINTRESOURCEW(1): winresource embeds the application icon as ID 1.
        let icon = LoadIconW(module, std::ptr::dangling::<u16>());
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: module,
            hIcon: icon,
            lpszClassName: class.as_ptr(),
            ..zeroed()
        };
        if RegisterClassW(&window_class) == 0 {
            return Err("RegisterClassW gagal".into());
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            0,
            0,
            DOCK_WIDTH,
            DOCK_HEIGHT,
            null_mut(),
            null_mut(),
            module,
            null(),
        );
        if hwnd.is_null() {
            return Err("CreateWindowExW gagal".into());
        }

        let mut app = Box::new(App {
            settings,
            settings_path,
            tray_open: false,
            icon,
            taskbar: None,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *app as *mut App) as isize);
        apply_glass(hwnd, &mut app);
        position_window(hwnd, false);
        ShowWindow(hwnd, SW_SHOW);
        RegisterHotKey(hwnd, MENU_HOTKEY_ID, MOD_CONTROL | MOD_ALT, b'D' as u32);
        SetTimer(hwnd, TIMER_ID, 30_000, None);

        if app.settings.hide_taskbar {
            // If the watcher cannot start, keep Windows' taskbar available.
            match TaskbarGuard::hide() {
                Ok(guard) => app.taskbar = guard,
                Err(error) => {
                    DestroyWindow(hwnd);
                    return Err(error);
                }
            }
        }

        let mut message: MSG = zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        drop(app);
    }
    Ok(())
}

fn watchdog(args: &[String]) -> Result<(), String> {
    let pid: u32 = args
        .first()
        .ok_or("PID watchdog tidak ada")?
        .parse()
        .map_err(|_| "PID watchdog tidak valid")?;
    let handles: Vec<usize> = args[1..]
        .iter()
        .map(|value| {
            value
                .parse()
                .map_err(|_| "handle taskbar tidak valid".to_string())
        })
        .collect::<Result<_, _>>()?;
    unsafe {
        let parent = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if parent.is_null() {
            return Err("OpenProcess untuk watchdog gagal".into());
        }
        println!("READY");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        WaitForSingleObject(parent, u32::MAX);
        restore_taskbars(&handles);
        CloseHandle(parent);
    }
    Ok(())
}

unsafe fn visible_taskbars() -> Vec<usize> {
    unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> i32 {
        unsafe {
            let mut name = [0u16; 64];
            let length = GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32);
            let class = String::from_utf16_lossy(&name[..length.max(0) as usize]);
            if (class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd")
                && IsWindowVisible(hwnd) != 0
            {
                (*(param as *mut Vec<usize>)).push(hwnd as usize);
            }
        }
        1
    }
    let mut handles = Vec::<usize>::new();
    unsafe { EnumWindows(Some(collect), (&mut handles as *mut Vec<usize>) as isize) };
    handles
}

unsafe fn restore_taskbars(handles: &[usize]) {
    for handle in handles {
        unsafe {
            let hwnd = *handle as HWND;
            let mut name = [0u16; 64];
            let length = GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32);
            let class = String::from_utf16_lossy(&name[..length.max(0) as usize]);
            if class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd" {
                ShowWindow(hwnd, SW_SHOW);
            }
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    unsafe {
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
        if pointer.is_null() {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        let app = &mut *pointer;
        match message {
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                paint(hwnd, app);
                0
            }
            WM_LBUTTONUP => {
                let x = (lparam as u32 & 0xffff) as i16 as i32;
                let y = ((lparam as u32 >> 16) & 0xffff) as i16 as i32;
                let dock_top = if app.tray_open { TRAY_HEIGHT } else { 0 };
                if app.settings.start && (158..241).contains(&x) && y >= dock_top {
                    open_start_menu();
                } else if app.settings.tray && x > DOCK_WIDTH - 116 && y >= dock_top {
                    app.tray_open = !app.tray_open;
                    position_window(hwnd, app.tray_open);
                    InvalidateRect(hwnd, null(), 0);
                }
                0
            }
            WM_RBUTTONUP | WM_HOTKEY => {
                show_settings_menu(hwnd, app);
                0
            }
            WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
                app.tray_open = false;
                position_window(hwnd, false);
                InvalidateRect(hwnd, null(), 0);
                0
            }
            WM_TIMER | WM_SETTINGCHANGE => {
                InvalidateRect(hwnd, null(), 0);
                0
            }
            WM_DISPLAYCHANGE => {
                position_window(hwnd, app.tray_open);
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                UnregisterHotKey(hwnd, MENU_HOTKEY_ID);
                KillTimer(hwnd, TIMER_ID);
                app.taskbar = None;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}

unsafe fn show_settings_menu(hwnd: HWND, app: &mut App) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        for (id, setting, label) in SETTINGS_MENU {
            let flags = MF_STRING
                | if app.settings.enabled(*setting) {
                    MF_CHECKED
                } else {
                    0
                };
            AppendMenuW(menu, flags, *id as usize, wide(label).as_ptr());
        }
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, 100, wide("Keluar dari theDOCK").as_ptr());
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let id = TrackPopupMenu(menu, TPM_RETURNCMD, point.x, point.y, 0, hwnd, null());
        DestroyMenu(menu);
        if id == 100 {
            DestroyWindow(hwnd);
        } else if let Some((_, setting, _)) =
            SETTINGS_MENU.iter().find(|(key, _, _)| *key == id as u32)
        {
            app.settings.toggle(*setting);
            if *setting == Setting::HideTaskbar {
                if app.settings.hide_taskbar {
                    match TaskbarGuard::hide() {
                        Ok(guard) => app.taskbar = guard,
                        Err(error) => {
                            app.settings.hide_taskbar = false;
                            show_error(&error);
                        }
                    }
                } else {
                    app.taskbar = None;
                }
            }
            if *setting == Setting::Tray && !app.settings.tray {
                app.tray_open = false;
                position_window(hwnd, false);
            }
            if *setting == Setting::Glass {
                apply_glass(hwnd, app);
            }
            if let Err(error) = app.settings.save(&app.settings_path) {
                show_error(&format!("Pengaturan gagal disimpan: {error}"));
            }
            InvalidateRect(hwnd, null(), 0);
        }
    }
}

unsafe fn apply_glass(hwnd: HWND, app: &mut App) {
    unsafe {
        let dark_mode: i32 = 1;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&dark_mode as *const i32).cast::<c_void>(),
            size_of::<i32>() as u32,
        );
        let backdrop = if app.settings.glass {
            DWMSBT_TRANSIENTWINDOW
        } else {
            DWMSBT_NONE
        };
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE as u32,
            (&backdrop as *const i32).cast::<c_void>(),
            size_of::<i32>() as u32,
        );
        let rounded = DWMWCP_ROUND;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&rounded as *const i32).cast::<c_void>(),
            size_of::<i32>() as u32,
        );
        // A full DWM frame made the GDI text white on a white client area on
        // some Windows themes. Paint an explicit dark surface, then give the
        // entire window modest translucency when glass is enabled.
        SetLayeredWindowAttributes(
            hwnd,
            0,
            if app.settings.glass { 235 } else { 255 },
            LWA_ALPHA,
        );
    }
}

unsafe fn position_window(hwnd: HWND, tray_open: bool) {
    unsafe {
        let height = DOCK_HEIGHT + if tray_open { TRAY_HEIGHT } else { 0 };
        let x = (GetSystemMetrics(SM_CXSCREEN) - DOCK_WIDTH) / 2;
        let y = GetSystemMetrics(SM_CYSCREEN) - height - 16;
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, DOCK_WIDTH, height, SWP_NOACTIVATE);
        let region = CreateRoundRectRgn(
            0,
            if tray_open { TRAY_HEIGHT } else { 0 },
            DOCK_WIDTH,
            height,
            66,
            66,
        );
        if !region.is_null() {
            if tray_open {
                let tray =
                    CreateRoundRectRgn(DOCK_WIDTH - 320, 0, DOCK_WIDTH, TRAY_HEIGHT + 20, 32, 32);
                if !tray.is_null() {
                    CombineRgn(region, region, tray, RGN_OR);
                    DeleteObject(tray);
                }
            }
            if SetWindowRgn(hwnd, region, 1) == 0 {
                DeleteObject(region);
            }
        }
    }
}

unsafe fn paint(hwnd: HWND, app: &App) {
    unsafe {
        let mut paint: PAINTSTRUCT = zeroed();
        let dc = BeginPaint(hwnd, &mut paint);
        let mut area: RECT = zeroed();
        GetClientRect(hwnd, &mut area);
        let background = CreateSolidBrush(rgb(18, 29, 51));
        FillRect(dc, &area, background);
        DeleteObject(background);
        let font = CreateFontW(
            -17,
            0,
            0,
            0,
            FW_MEDIUM as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide("Segoe UI").as_ptr(),
        );
        let font = if font.is_null() {
            GetStockObject(DEFAULT_GUI_FONT)
        } else {
            font
        };
        let old_font = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, rgb(245, 247, 255));

        let dock_top = if app.tray_open { TRAY_HEIGHT } else { 0 };
        capsule(
            dc,
            4,
            dock_top + 4,
            DOCK_WIDTH - 4,
            dock_top + 62,
            rgb(31, 49, 83),
        );
        if !app.icon.is_null() {
            DrawIconEx(
                dc,
                18,
                dock_top + 17,
                app.icon,
                32,
                32,
                0,
                null_mut(),
                DI_NORMAL,
            );
            label(dc, 57, dock_top + 25, "theDOCK");
        } else {
            label(dc, 24, dock_top + 25, "theDOCK");
        }

        if app.settings.start {
            capsule(dc, 158, dock_top + 12, 240, dock_top + 54, rgb(50, 75, 118));
            label(dc, 178, dock_top + 26, "Start");
        }

        let mut x = 260;
        let mut clock: SYSTEMTIME = zeroed();
        GetLocalTime(&mut clock);
        if app.settings.clock {
            label(
                dc,
                x,
                dock_top + 25,
                &format!("{:02}:{:02}", clock.wHour, clock.wMinute),
            );
            x += 62;
        }
        if app.settings.date {
            label(
                dc,
                x,
                dock_top + 25,
                &format!("{:02}/{:02}", clock.wDay, clock.wMonth),
            );
            x += 68;
        }
        if app.settings.ram {
            let mut memory: MEMORYSTATUSEX = zeroed();
            memory.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut memory) != 0 {
                label(
                    dc,
                    x,
                    dock_top + 25,
                    &format!("RAM {}%", memory.dwMemoryLoad),
                );
                x += 84;
            }
        }
        if app.settings.battery {
            let mut power: SYSTEM_POWER_STATUS = zeroed();
            if GetSystemPowerStatus(&mut power) != 0 && power.BatteryLifePercent != 255 {
                label(
                    dc,
                    x,
                    dock_top + 25,
                    &format!("BAT {}%", power.BatteryLifePercent),
                );
            }
        }
        if app.settings.tray {
            capsule(
                dc,
                DOCK_WIDTH - 114,
                dock_top + 12,
                DOCK_WIDTH - 18,
                dock_top + 54,
                rgb(50, 75, 118),
            );
            label(dc, DOCK_WIDTH - 91, dock_top + 26, "Tray ^");
        }
        if app.tray_open {
            capsule(
                dc,
                DOCK_WIDTH - 310,
                4,
                DOCK_WIDTH - 8,
                TRAY_HEIGHT - 2,
                rgb(31, 49, 83),
            );
            label(dc, DOCK_WIDTH - 286, 27, "Panel Tray");
            label(
                dc,
                DOCK_WIDTH - 286,
                57,
                "Ikon aplikasi lain belum tersedia.",
            );
            label(dc, DOCK_WIDTH - 286, 79, "Prototipe Windows diperlukan.");
        }
        SelectObject(dc, old_font);
        if font != GetStockObject(DEFAULT_GUI_FONT) {
            DeleteObject(font);
        }
        EndPaint(hwnd, &paint);
    }
}

unsafe fn open_start_menu() {
    let down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_LWIN,
                ..KEYBDINPUT::default()
            },
        },
    };
    let up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_LWIN,
                dwFlags: KEYEVENTF_KEYUP,
                ..KEYBDINPUT::default()
            },
        },
    };
    unsafe { SendInput(2, [down, up].as_ptr(), size_of::<INPUT>() as i32) };
}

unsafe fn capsule(dc: HDC, left: i32, top: i32, right: i32, bottom: i32, color: u32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        let previous = SelectObject(dc, brush);
        RoundRect(dc, left, top, right, bottom, 28, 28);
        SelectObject(dc, previous);
        DeleteObject(brush);
    }
}

unsafe fn label(dc: HDC, x: i32, y: i32, text: &str) {
    let wide = wide(text);
    unsafe { TextOutW(dc, x, y, wide.as_ptr(), (wide.len() - 1) as i32) };
}

fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    red as u32 | (green as u32) << 8 | (blue as u32) << 16
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn show_error(message: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(message).as_ptr(),
            wide("theDOCK").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
