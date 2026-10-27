//! A small native Win32 shell for the first prototype.
//! Safety: Win32 handles and callback pointers stay on this UI thread.

use std::ffi::c_void;
use std::io::{BufRead, BufReader};
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};
use thedock::{DockWidth, Setting, Settings};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, FILETIME, GetLastError, HANDLE, HWND, LPARAM, POINT, RECT,
    SYSTEMTIME,
};
use windows_sys::Win32::Graphics::Dwm::{
    DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CombineRgn, CreateFontW, CreateRectRgn,
    CreateRoundRectRgn, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_GUI_FONT, DEFAULT_PITCH,
    DeleteObject, EndPaint, FW_MEDIUM, FW_SEMIBOLD, FillRect, GRADIENT_FILL_RECT_V, GRADIENT_RECT,
    GetStockObject, GradientFill, HDC, InvalidateRect, NULL_PEN, OUT_DEFAULT_PRECIS, PAINTSTRUCT,
    RGN_OR, RestoreDC, RoundRect, SaveDC, SelectClipRgn, SelectObject, SetBkMode, SetTextColor,
    SetWindowRgn, TRANSPARENT, TRIVERTEX, TextOutW,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, SearchPathW};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows_sys::Win32::System::SystemInformation::{
    GetLocalTime, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcessId, GetSystemTimes, OpenProcess,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, MOD_ALT, MOD_CONTROL,
    RegisterHotKey, SendInput, UnregisterHotKey, VK_ESCAPE, VK_LWIN, VK_MEDIA_NEXT_TRACK,
    VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};
use windows_sys::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, HDROP, SHFILEINFOW, SHGFI_ICON,
    SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DI_NORMAL, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, DispatchMessageW, DrawIconEx, EnumWindows, GCLP_HICON,
    GCLP_HICONSM, GW_OWNER, GWLP_USERDATA, GetClassLongPtrW, GetClassNameW, GetClientRect,
    GetCursorPos, GetMessageW, GetSystemMetrics, GetWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, HICON, HWND_TOPMOST, IDI_APPLICATION, IsIconic,
    IsWindowVisible, KillTimer, LWA_ALPHA, LoadIconW, MB_ICONERROR, MB_OK, MF_CHECKED,
    MF_SEPARATOR, MF_STRING, MSG, MessageBoxW, PostQuitMessage, RegisterClassW, SM_CXSCREEN,
    SM_CYSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SetForegroundWindow, SetLayeredWindowAttributes,
    SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TPM_RETURNCMD, TrackPopupMenu,
    TranslateMessage, WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE, WM_DROPFILES, WM_ERASEBKGND,
    WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONUP, WM_SETTINGCHANGE,
    WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const CLASS_NAME: &str = "theDOCK_prototype";
const DOCK_MAX_WIDTH: i32 = 1120;
const DOCK_HEIGHT: i32 = 66;
const TRAY_HEIGHT: i32 = 128;
const SETTINGS_HEIGHT: i32 = 400;
const TIMER_ID: usize = 1;
const AUTO_HIDE_TIMER_ID: usize = 2;
const MENU_HOTKEY_ID: i32 = 1;
const SETTINGS_MENU: &[(u32, Setting, &str)] = &[
    (8, Setting::Start, "Tombol Start"),
    (9, Setting::PinnedApps, "Aplikasi dipin"),
    (10, Setting::RunningApps, "Aplikasi berjalan"),
    (1, Setting::Clock, "Jam"),
    (2, Setting::Date, "Tanggal"),
    (11, Setting::Calendar, "Kalender"),
    (12, Setting::Music, "Kontrol musik"),
    (13, Setting::Cpu, "CPU"),
    (3, Setting::Ram, "RAM"),
    (4, Setting::Battery, "Baterai"),
    (5, Setting::Tray, "Tombol Tray"),
    (17, Setting::TrayVolume, "Tray: volume"),
    (18, Setting::TrayNetwork, "Tray: jaringan"),
    (19, Setting::TrayBluetooth, "Tray: Bluetooth"),
    (6, Setting::Glass, "Efek kaca"),
    (14, Setting::LightGlass, "Kaca terang"),
    (7, Setting::HideTaskbar, "Sembunyikan taskbar"),
    (15, Setting::DockTop, "Posisi di atas"),
    (16, Setting::AutoHide, "Sembunyi otomatis"),
];

struct App {
    settings: Settings,
    settings_path: PathBuf,
    tray_open: bool,
    settings_open: bool,
    icon: HICON,
    running: Vec<RunningWindow>,
    hits: Vec<HitBox>,
    cpu_sample: Option<(u64, u64)>,
    cpu_percent: Option<u32>,
    auto_hidden: bool,
    last_hover: Instant,
    menu_open: bool,
    taskbar: Option<TaskbarGuard>,
}

#[derive(Clone)]
struct RunningWindow {
    hwnd: usize,
    title: String,
}

#[derive(Clone, Copy)]
enum HitAction {
    Start,
    Pin(usize),
    Running(usize),
    MoreApps,
    Tray,
    Settings,
    ToggleSetting(Setting),
    Calendar,
    MusicPrev,
    MusicPlay,
    MusicNext,
    VolumeDown,
    VolumeMute,
    VolumeUp,
    Network,
    Bluetooth,
}

struct HitBox {
    rect: RECT,
    action: HitAction,
}

impl HitBox {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.rect.left && x < self.rect.right && y >= self.rect.top && y < self.rect.bottom
    }
}

impl App {
    fn panel_height(&self) -> i32 {
        if self.settings_open {
            SETTINGS_HEIGHT
        } else if self.tray_open {
            TRAY_HEIGHT
        } else {
            0
        }
    }

    fn sample_cpu(&mut self) {
        unsafe {
            let mut idle: FILETIME = zeroed();
            let mut kernel: FILETIME = zeroed();
            let mut user: FILETIME = zeroed();
            if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
                return;
            }
            let idle = filetime_ticks(idle);
            let total = filetime_ticks(kernel).saturating_add(filetime_ticks(user));
            if let Some((previous_idle, previous_total)) = self.cpu_sample {
                let idle_delta = idle.saturating_sub(previous_idle);
                let total_delta = total.saturating_sub(previous_total);
                let busy = total_delta.saturating_sub(idle_delta).saturating_mul(100);
                self.cpu_percent = busy
                    .checked_div(total_delta)
                    .map(|value| value.min(100) as u32);
            }
            self.cpu_sample = Some((idle, total));
        }
    }
}

fn filetime_ticks(value: FILETIME) -> u64 {
    (value.dwHighDateTime as u64) << 32 | value.dwLowDateTime as u64
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
        let helper = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("thedock-watchdog.exe");
        if !helper.is_file() {
            return Err("thedock-watchdog.exe tidak ditemukan; taskbar tetap terlihat".into());
        }
        let mut command = Command::new(helper);
        command
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
    let settings = match Settings::load_checked(&settings_path) {
        Ok(settings) => settings,
        Err(error) => {
            let safe = Settings {
                hide_taskbar: false,
                ..Settings::default()
            };
            show_error(&format!(
                "Pengaturan tidak dapat dibaca ({error}). Taskbar tetap terlihat."
            ));
            safe
        }
    };

    unsafe {
        let class = wide(CLASS_NAME);
        let title = wide("theDOCK");
        let module = GetModuleHandleW(null());
        if module.is_null() {
            return Err("GetModuleHandleW gagal".into());
        }
        // MAKEINTRESOURCEW(1): integer resource identifiers are encoded as a
        // pointer value, not as a pointer to an integer in memory.
        let icon = LoadIconW(module, std::ptr::without_provenance::<u16>(1));
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
            DOCK_MAX_WIDTH,
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
            settings_open: false,
            icon,
            running: Vec::new(),
            hits: Vec::new(),
            cpu_sample: None,
            cpu_percent: None,
            auto_hidden: false,
            last_hover: Instant::now(),
            menu_open: false,
            taskbar: None,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *app as *mut App) as isize);
        apply_glass(hwnd, &mut app);
        app.running = running_windows(hwnd);
        app.sample_cpu();
        position_window(hwnd, &app);
        ShowWindow(hwnd, SW_SHOW);
        DragAcceptFiles(hwnd, 1);
        RegisterHotKey(hwnd, MENU_HOTKEY_ID, MOD_CONTROL | MOD_ALT, b'D' as u32);
        SetTimer(hwnd, TIMER_ID, 10_000, None);
        if app.settings.auto_hide {
            SetTimer(hwnd, AUTO_HIDE_TIMER_ID, 500, None);
        }

        if app.settings.hide_taskbar {
            // If the watcher cannot start, keep Windows' taskbar available.
            match TaskbarGuard::hide() {
                Ok(guard) => app.taskbar = guard,
                Err(error) => {
                    app.settings.hide_taskbar = false;
                    show_error(&error);
                    save_settings(hwnd, &app);
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

unsafe fn running_windows(own_window: HWND) -> Vec<RunningWindow> {
    struct Collector {
        own_window: HWND,
        windows: Vec<RunningWindow>,
    }

    unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> i32 {
        unsafe {
            let collector = &mut *(param as *mut Collector);
            if hwnd == collector.own_window
                || IsWindowVisible(hwnd) == 0
                || !GetWindow(hwnd, GW_OWNER).is_null()
            {
                return 1;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == GetCurrentProcessId() {
                return 1;
            }
            let mut title = [0u16; 256];
            let length = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
            if length <= 0 {
                return 1;
            }
            let mut class = [0u16; 64];
            let class_length = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
            let class = String::from_utf16_lossy(&class[..class_length.max(0) as usize]);
            if matches!(
                class.as_str(),
                "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "Progman" | "WorkerW"
            ) {
                return 1;
            }
            collector.windows.push(RunningWindow {
                hwnd: hwnd as usize,
                title: String::from_utf16_lossy(&title[..length as usize]),
            });
        }
        1
    }

    let mut collector = Collector {
        own_window,
        windows: Vec::new(),
    };
    unsafe { EnumWindows(Some(collect), (&mut collector as *mut Collector) as isize) };
    collector.windows
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
                let action = app
                    .hits
                    .iter()
                    .find(|hit| hit.contains(x, y))
                    .map(|hit| hit.action);
                match action {
                    Some(HitAction::Start) => open_start_menu(),
                    Some(HitAction::Pin(index)) => launch_pin(hwnd, app, index),
                    Some(HitAction::Running(index)) => activate_window(app, index),
                    Some(HitAction::MoreApps) => show_overflow_menu(hwnd, app),
                    Some(HitAction::Tray) => {
                        app.tray_open = !app.tray_open;
                        app.settings_open = false;
                        position_window(hwnd, app);
                        InvalidateRect(hwnd, null(), 0);
                    }
                    Some(HitAction::Settings) => {
                        app.settings_open = !app.settings_open;
                        app.tray_open = false;
                        position_window(hwnd, app);
                        InvalidateRect(hwnd, null(), 0);
                    }
                    Some(HitAction::ToggleSetting(setting)) => toggle_setting(hwnd, app, setting),
                    Some(HitAction::Calendar) => {
                        app.menu_open = true;
                        show_calendar(hwnd);
                        app.menu_open = false;
                    }
                    Some(HitAction::MusicPrev) => send_key(VK_MEDIA_PREV_TRACK),
                    Some(HitAction::MusicPlay) => send_key(VK_MEDIA_PLAY_PAUSE),
                    Some(HitAction::MusicNext) => send_key(VK_MEDIA_NEXT_TRACK),
                    Some(HitAction::VolumeDown) => send_key(VK_VOLUME_DOWN),
                    Some(HitAction::VolumeMute) => send_key(VK_VOLUME_MUTE),
                    Some(HitAction::VolumeUp) => send_key(VK_VOLUME_UP),
                    Some(HitAction::Network) => launch_uri(hwnd, "ms-settings:network-status"),
                    Some(HitAction::Bluetooth) => launch_uri(hwnd, "ms-settings:bluetooth"),
                    None => {}
                }
                0
            }
            WM_RBUTTONUP => {
                let x = (lparam as u32 & 0xffff) as i16 as i32;
                let y = ((lparam as u32 >> 16) & 0xffff) as i16 as i32;
                if let Some(HitAction::Pin(index)) = app
                    .hits
                    .iter()
                    .find(|hit| hit.contains(x, y))
                    .map(|hit| hit.action)
                {
                    show_pin_menu(hwnd, app, index);
                } else {
                    show_settings_menu(hwnd, app);
                }
                0
            }
            WM_HOTKEY => {
                app.settings_open = !app.settings_open;
                app.tray_open = false;
                position_window(hwnd, app);
                InvalidateRect(hwnd, null(), 0);
                0
            }
            WM_DROPFILES => {
                pin_dropped_files(hwnd, app, wparam as HDROP);
                0
            }
            WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
                app.tray_open = false;
                app.settings_open = false;
                position_window(hwnd, app);
                InvalidateRect(hwnd, null(), 0);
                0
            }
            WM_MOUSEMOVE => {
                app.last_hover = Instant::now();
                if app.auto_hidden {
                    app.auto_hidden = false;
                    position_window(hwnd, app);
                    InvalidateRect(hwnd, null(), 0);
                }
                0
            }
            WM_TIMER => {
                if wparam == AUTO_HIDE_TIMER_ID {
                    auto_hide_tick(hwnd, app);
                } else {
                    app.running = running_windows(hwnd);
                    app.sample_cpu();
                    InvalidateRect(hwnd, null(), 0);
                }
                0
            }
            WM_SETTINGCHANGE => {
                InvalidateRect(hwnd, null(), 0);
                0
            }
            WM_DISPLAYCHANGE => {
                position_window(hwnd, app);
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                DragAcceptFiles(hwnd, 0);
                UnregisterHotKey(hwnd, MENU_HOTKEY_ID);
                KillTimer(hwnd, TIMER_ID);
                KillTimer(hwnd, AUTO_HIDE_TIMER_ID);
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
        for (id, width, label) in [
            (301, DockWidth::Narrow, "Lebar kecil"),
            (302, DockWidth::Normal, "Lebar sedang"),
            (303, DockWidth::Wide, "Lebar besar"),
        ] {
            let checked = if app.settings.dock_width == width {
                MF_CHECKED
            } else {
                0
            };
            AppendMenuW(menu, MF_STRING | checked, id, wide(label).as_ptr());
        }
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, 100, wide("Keluar dari theDOCK").as_ptr());
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        app.menu_open = true;
        let id = TrackPopupMenu(menu, TPM_RETURNCMD, point.x, point.y, 0, hwnd, null());
        app.menu_open = false;
        DestroyMenu(menu);
        if id == 100 {
            DestroyWindow(hwnd);
        } else if (301..=303).contains(&id) {
            app.settings.dock_width = match id {
                301 => DockWidth::Narrow,
                303 => DockWidth::Wide,
                _ => DockWidth::Normal,
            };
            position_window(hwnd, app);
            save_settings(hwnd, app);
        } else if let Some((_, setting, _)) =
            SETTINGS_MENU.iter().find(|(key, _, _)| *key == id as u32)
        {
            toggle_setting(hwnd, app, *setting);
        }
    }
}

unsafe fn toggle_setting(hwnd: HWND, app: &mut App, setting: Setting) {
    app.settings.toggle(setting);
    if setting == Setting::HideTaskbar {
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
    if setting == Setting::Tray && !app.settings.tray {
        app.tray_open = false;
    }
    if setting == Setting::Glass || setting == Setting::LightGlass {
        unsafe { apply_glass(hwnd, app) };
    }
    if setting == Setting::AutoHide {
        app.auto_hidden = false;
        app.last_hover = Instant::now();
        unsafe {
            if app.settings.auto_hide {
                SetTimer(hwnd, AUTO_HIDE_TIMER_ID, 500, None);
            } else {
                KillTimer(hwnd, AUTO_HIDE_TIMER_ID);
            }
        }
    }
    unsafe {
        position_window(hwnd, app);
        save_settings(hwnd, app);
    }
}

unsafe fn show_pin_menu(hwnd: HWND, app: &mut App, index: usize) {
    if index >= app.settings.pins.len() {
        return;
    }
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let title = format!("Buka {}", app.settings.pins[index].label);
        AppendMenuW(menu, MF_STRING, 201, wide(&title).as_ptr());
        AppendMenuW(menu, MF_STRING, 202, wide("Geser ke kiri").as_ptr());
        AppendMenuW(menu, MF_STRING, 203, wide("Geser ke kanan").as_ptr());
        AppendMenuW(menu, MF_STRING, 204, wide("Lepas pin").as_ptr());
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        app.menu_open = true;
        let action = TrackPopupMenu(menu, TPM_RETURNCMD, point.x, point.y, 0, hwnd, null());
        app.menu_open = false;
        DestroyMenu(menu);
        let changed = match action {
            201 => {
                launch_pin(hwnd, app, index);
                false
            }
            202 => app.settings.move_pin(index, -1),
            203 => app.settings.move_pin(index, 1),
            204 => app.settings.unpin(index),
            _ => false,
        };
        if changed {
            save_settings(hwnd, app);
        }
    }
}

unsafe fn show_overflow_menu(hwnd: HWND, app: &mut App) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        if app.settings.pinned_apps {
            for (index, pin) in app.settings.pins.iter().enumerate().take(100) {
                AppendMenuW(menu, MF_STRING, 1_000 + index, wide(&pin.label).as_ptr());
            }
        }
        if app.settings.running_apps {
            if !app.settings.pins.is_empty() {
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            }
            for (index, window) in app.running.iter().enumerate().take(100) {
                let name: String = window.title.chars().take(50).collect();
                AppendMenuW(menu, MF_STRING, 2_000 + index, wide(&name).as_ptr());
            }
        }
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        app.menu_open = true;
        let action = TrackPopupMenu(menu, TPM_RETURNCMD, point.x, point.y, 0, hwnd, null());
        app.menu_open = false;
        DestroyMenu(menu);
        match action {
            1_000..=1_099 => launch_pin(hwnd, app, (action - 1_000) as usize),
            2_000..=2_099 => activate_window(app, (action - 2_000) as usize),
            _ => {}
        }
    }
}

unsafe fn launch_pin(hwnd: HWND, app: &App, index: usize) {
    let Some(pin) = app.settings.pins.get(index) else {
        return;
    };
    unsafe {
        let result = ShellExecuteW(
            hwnd,
            wide("open").as_ptr(),
            wide(&pin.target).as_ptr(),
            null(),
            null(),
            SW_SHOW,
        );
        if result as isize <= 32 {
            show_error(&format!("Tidak dapat membuka {}", pin.label));
        }
    }
}

unsafe fn launch_uri(hwnd: HWND, uri: &str) {
    unsafe {
        let result = ShellExecuteW(
            hwnd,
            wide("open").as_ptr(),
            wide(uri).as_ptr(),
            null(),
            null(),
            SW_SHOW,
        );
        if result as isize <= 32 {
            show_error("Pengaturan Windows tidak dapat dibuka");
        }
    }
}

unsafe fn activate_window(app: &App, index: usize) {
    let Some(window) = app.running.get(index) else {
        return;
    };
    unsafe {
        let hwnd = window.hwnd as HWND;
        if IsIconic(hwnd) != 0 {
            ShowWindow(
                hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_RESTORE,
            );
        }
        SetForegroundWindow(hwnd);
    }
}

unsafe fn pin_dropped_files(hwnd: HWND, app: &mut App, drop: HDROP) {
    unsafe {
        let count = DragQueryFileW(drop, u32::MAX, null_mut(), 0).min(32);
        let mut changed = false;
        for index in 0..count {
            let length = DragQueryFileW(drop, index, null_mut(), 0);
            if length == 0 {
                continue;
            }
            let mut buffer = vec![0u16; length as usize + 1];
            if DragQueryFileW(drop, index, buffer.as_mut_ptr(), buffer.len() as u32) == 0 {
                continue;
            }
            let target = String::from_utf16_lossy(&buffer[..length as usize]);
            let label = Path::new(&target)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Aplikasi")
                .to_string();
            changed |= app.settings.pin(label, target);
        }
        DragFinish(drop);
        if changed {
            save_settings(hwnd, app);
        }
    }
}

unsafe fn save_settings(hwnd: HWND, app: &App) {
    if let Err(error) = app.settings.save(&app.settings_path) {
        show_error(&format!("Pengaturan gagal disimpan: {error}"));
    }
    unsafe { InvalidateRect(hwnd, null(), 0) };
}

unsafe fn show_calendar(hwnd: HWND) {
    unsafe {
        let mut date: SYSTEMTIME = zeroed();
        GetLocalTime(&mut date);
        let year = date.wYear as u32;
        let month = date.wMonth as u32;
        let first_weekday = (date.wDayOfWeek as u32 + 7 - ((date.wDay as u32 - 1) % 7)) % 7;
        let days = match month {
            4 | 6 | 9 | 11 => 30,
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29
            }
            2 => 28,
            _ => 31,
        };
        let mut text = format!("{:02}/{year}\nMin Sen Sel Rab Kam Jum Sab\n", month);
        for _ in 0..first_weekday {
            text.push_str("    ");
        }
        for day in 1..=days {
            text.push_str(&format!("{day:>3} "));
            if (first_weekday + day).is_multiple_of(7) {
                text.push('\n');
            }
        }
        MessageBoxW(
            hwnd,
            wide(&text).as_ptr(),
            wide("Kalender theDOCK").as_ptr(),
            MB_OK,
        );
    }
}

unsafe fn apply_glass(hwnd: HWND, app: &mut App) {
    unsafe {
        // Both glass palettes are dark. A light DWM backdrop left bright
        // corners around the dark custom-painted surface on Windows 11.
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

unsafe fn auto_hide_tick(hwnd: HWND, app: &mut App) {
    if !app.settings.auto_hide
        || app.auto_hidden
        || app.tray_open
        || app.settings_open
        || app.menu_open
    {
        return;
    }
    unsafe {
        let mut cursor: POINT = zeroed();
        let mut window: RECT = zeroed();
        if GetCursorPos(&mut cursor) == 0 || GetWindowRect(hwnd, &mut window) == 0 {
            return;
        }
        if cursor.x >= window.left - 4
            && cursor.x < window.right + 4
            && cursor.y >= window.top - 4
            && cursor.y < window.bottom + 4
        {
            app.last_hover = Instant::now();
        } else if app.last_hover.elapsed() >= Duration::from_millis(1200) {
            app.auto_hidden = true;
            position_window(hwnd, app);
            InvalidateRect(hwnd, null(), 0);
        }
    }
}

unsafe fn position_window(hwnd: HWND, app: &App) {
    unsafe {
        let hidden = app.settings.auto_hide && app.auto_hidden;
        let panel_height = if hidden { 0 } else { app.panel_height() };
        let panel = panel_height > 0;
        let height = if hidden {
            4
        } else {
            DOCK_HEIGHT + panel_height
        };
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let width = (screen_width - 32).clamp(560, app.settings.dock_width.pixels());
        let x = (screen_width - width) / 2;
        let y = if app.settings.dock_top {
            if hidden { 0 } else { 16 }
        } else {
            GetSystemMetrics(SM_CYSCREEN) - height - if hidden { 0 } else { 16 }
        };
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE);
        let dock_top = if panel && !app.settings.dock_top {
            panel_height
        } else {
            0
        };
        let region = if hidden {
            CreateRectRgn(0, 0, width, 4)
        } else {
            CreateRoundRectRgn(0, dock_top, width, dock_top + DOCK_HEIGHT, 38, 38)
        };
        if !region.is_null() {
            if panel {
                let panel_y = if app.settings.dock_top {
                    DOCK_HEIGHT - 20
                } else {
                    0
                };
                let panel_width = if app.settings_open { 570 } else { 320 };
                let tray = CreateRoundRectRgn(
                    width - panel_width,
                    panel_y,
                    width,
                    panel_y + panel_height + 20,
                    32,
                    32,
                );
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

unsafe fn paint(hwnd: HWND, app: &mut App) {
    unsafe {
        let mut paint: PAINTSTRUCT = zeroed();
        let dc = BeginPaint(hwnd, &mut paint);
        let mut area: RECT = zeroed();
        GetClientRect(hwnd, &mut area);
        let (background_color, surface_color, button_color, pin_color, running_color, text_color) =
            if app.settings.light_glass {
                (
                    rgb(21, 31, 48),
                    rgb(35, 51, 75),
                    rgb(47, 67, 97),
                    rgb(42, 61, 91),
                    rgb(49, 75, 111),
                    rgb(242, 247, 255),
                )
            } else {
                (
                    rgb(5, 9, 18),
                    rgb(12, 19, 33),
                    rgb(20, 30, 49),
                    rgb(18, 28, 46),
                    rgb(22, 37, 60),
                    rgb(246, 249, 255),
                )
            };
        let background = CreateSolidBrush(background_color);
        FillRect(dc, &area, background);
        DeleteObject(background);
        app.hits.clear();
        if app.settings.auto_hide && app.auto_hidden {
            EndPaint(hwnd, &paint);
            return;
        }
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
        SetTextColor(dc, text_color);

        let width = area.right;
        let dock_top = if app.panel_height() > 0 && !app.settings.dock_top {
            app.panel_height()
        } else {
            0
        };
        capsule(dc, 4, dock_top + 4, width - 4, dock_top + 62, surface_color);
        if !app.icon.is_null() {
            DrawIconEx(
                dc,
                16,
                dock_top + 17,
                app.icon,
                32,
                32,
                0,
                null_mut(),
                DI_NORMAL,
            );
        } else {
            label(dc, 20, dock_top + 25, "D");
        }
        capsule(dc, 56, dock_top + 16, 90, dock_top + 50, button_color);
        label(dc, 63, dock_top + 23, "⚙");
        app.hits.push(HitBox {
            rect: rect(56, dock_top + 16, 90, dock_top + 50),
            action: HitAction::Settings,
        });

        if app.settings.start {
            capsule(dc, 100, dock_top + 12, 144, dock_top + 54, button_color);
            label(dc, 112, dock_top + 24, "⊞");
            app.hits.push(HitBox {
                rect: rect(100, dock_top + 12, 144, dock_top + 54),
                action: HitAction::Start,
            });
        }

        let mut right = width - 18;
        if app.settings.clock || app.settings.date {
            let card_width = if app.settings.clock && app.settings.date {
                174
            } else {
                92
            };
            right -= card_width;
            draw_clock_card(
                dc,
                app,
                right,
                dock_top,
                card_width,
                button_color,
                text_color,
            );
            right -= 10;
        }
        if app.settings.tray {
            let left = right - 48;
            capsule(dc, left, dock_top + 12, right, dock_top + 54, button_color);
            label(
                dc,
                left + 17,
                dock_top + 24,
                if app.tray_open { "⌄" } else { "⌃" },
            );
            app.hits.push(HitBox {
                rect: rect(left, dock_top + 12, right, dock_top + 54),
                action: HitAction::Tray,
            });
            right = left - 10;
        }

        let reserve_apps = 160 + 96;
        if app.settings.battery && right - 88 >= reserve_apps {
            let mut power: SYSTEM_POWER_STATUS = zeroed();
            let value = if GetSystemPowerStatus(&mut power) != 0 && power.BatteryLifePercent != 255
            {
                format!("BAT {}%", power.BatteryLifePercent)
            } else {
                "AC".into()
            };
            right -= 82;
            capsule(
                dc,
                right - 5,
                dock_top + 12,
                right + 78,
                dock_top + 54,
                button_color,
            );
            label(dc, right, dock_top + 25, &value);
            right -= 6;
        }
        if app.settings.ram && right - 94 >= reserve_apps {
            let mut memory: MEMORYSTATUSEX = zeroed();
            memory.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            let value = if GlobalMemoryStatusEx(&mut memory) != 0 {
                format!("RAM {}%", memory.dwMemoryLoad)
            } else {
                "RAM ?".into()
            };
            right -= 88;
            capsule(
                dc,
                right - 5,
                dock_top + 12,
                right + 84,
                dock_top + 54,
                button_color,
            );
            label(dc, right, dock_top + 25, &value);
            right -= 6;
        }
        if app.settings.cpu && right - 84 >= reserve_apps {
            right -= 78;
            let value = app
                .cpu_percent
                .map_or("CPU ?".into(), |value| format!("CPU {value}%"));
            capsule(
                dc,
                right - 5,
                dock_top + 12,
                right + 73,
                dock_top + 54,
                button_color,
            );
            label(dc, right, dock_top + 25, &value);
            right -= 6;
        }
        if app.settings.music && right - 126 >= reserve_apps {
            right -= 118;
            capsule(
                dc,
                right,
                dock_top + 12,
                right + 118,
                dock_top + 54,
                button_color,
            );
            for (offset, label_text, action) in [
                (0, "⏮", HitAction::MusicPrev),
                (40, "⏯", HitAction::MusicPlay),
                (80, "⏭", HitAction::MusicNext),
            ] {
                let left = right + offset;
                label(dc, left + 11, dock_top + 24, label_text);
                app.hits.push(HitBox {
                    rect: rect(left, dock_top + 12, left + 38, dock_top + 54),
                    action,
                });
            }
            right -= 8;
        }

        let divider = CreateSolidBrush(if app.settings.light_glass {
            rgb(139, 168, 210)
        } else {
            rgb(105, 145, 216)
        });
        FillRect(dc, &rect(152, dock_top + 16, 153, dock_top + 50), divider);
        DeleteObject(divider);

        let mut x = 162;
        let total = (if app.settings.pinned_apps {
            app.settings.pins.len()
        } else {
            0
        }) + (if app.settings.running_apps {
            app.running.len()
        } else {
            0
        });
        let capacity = ((right - 8 - x) / 48).max(0) as usize;
        let visible_capacity = if total > capacity {
            capacity.saturating_sub(1)
        } else {
            capacity
        };
        let mut shown = 0;
        if app.settings.pinned_apps {
            for (index, pin) in app.settings.pins.iter().enumerate() {
                if shown >= visible_capacity {
                    break;
                }
                capsule(dc, x, dock_top + 10, x + 42, dock_top + 56, pin_color);
                draw_pin_icon(dc, &pin.target, x + 6, dock_top + 17, &pin.label);
                app.hits.push(HitBox {
                    rect: rect(x, dock_top + 10, x + 42, dock_top + 56),
                    action: HitAction::Pin(index),
                });
                x += 48;
                shown += 1;
            }
        }
        if app.settings.running_apps {
            for (index, window) in app.running.iter().enumerate() {
                if shown >= visible_capacity {
                    break;
                }
                capsule(dc, x, dock_top + 10, x + 42, dock_top + 56, running_color);
                let icon = GetClassLongPtrW(window.hwnd as HWND, GCLP_HICON);
                let icon = if icon == 0 {
                    GetClassLongPtrW(window.hwnd as HWND, GCLP_HICONSM)
                } else {
                    icon
                };
                if icon != 0 {
                    DrawIconEx(
                        dc,
                        x + 6,
                        dock_top + 17,
                        icon as HICON,
                        28,
                        28,
                        0,
                        null_mut(),
                        DI_NORMAL,
                    );
                } else {
                    draw_process_icon(dc, window.hwnd as HWND, x + 6, dock_top + 17);
                }
                capsule(
                    dc,
                    x + 19,
                    dock_top + 55,
                    x + 24,
                    dock_top + 60,
                    rgb(68, 163, 255),
                );
                app.hits.push(HitBox {
                    rect: rect(x, dock_top + 10, x + 42, dock_top + 56),
                    action: HitAction::Running(index),
                });
                x += 48;
                shown += 1;
            }
        }
        if total > shown && capacity > 0 {
            capsule(dc, x, dock_top + 10, x + 42, dock_top + 56, button_color);
            label(dc, x + 13, dock_top + 25, "···");
            app.hits.push(HitBox {
                rect: rect(x, dock_top + 10, x + 42, dock_top + 56),
                action: HitAction::MoreApps,
            });
        } else if total == 0 && right - x > 160 {
            label(dc, x + 5, dock_top + 25, "Tarik aplikasi ke dock untuk pin");
        }

        if app.tray_open {
            let panel_top = if app.settings.dock_top {
                DOCK_HEIGHT - 16
            } else {
                4
            };
            capsule(
                dc,
                width - 310,
                panel_top,
                width - 8,
                panel_top + TRAY_HEIGHT - 2,
                surface_color,
            );
            label(dc, width - 286, panel_top + 23, "Panel Tray");
            let mut controls = Vec::new();
            if app.settings.tray_volume {
                controls.extend([
                    (HitAction::VolumeDown, "Vol-"),
                    (HitAction::VolumeMute, "Mute"),
                    (HitAction::VolumeUp, "Vol+"),
                ]);
            }
            if app.settings.tray_network {
                controls.push((HitAction::Network, "Net"));
            }
            if app.settings.tray_bluetooth {
                controls.push((HitAction::Bluetooth, "BT"));
            }
            for (index, (action, label_text)) in controls.into_iter().enumerate() {
                let x = width - 298 + index as i32 * 55;
                let top = panel_top + 40;
                capsule(dc, x, top, x + 50, top + 40, button_color);
                label(dc, x + 6, top + 13, label_text);
                app.hits.push(HitBox {
                    rect: rect(x, top, x + 50, top + 40),
                    action,
                });
            }
            label(
                dc,
                width - 286,
                panel_top + 100,
                "Ikon aplikasi lain: belum tersedia",
            );
        }
        if app.settings_open {
            draw_settings_panel(dc, app, width, surface_color, button_color, text_color);
        }
        SelectObject(dc, old_font);
        if font != GetStockObject(DEFAULT_GUI_FONT) {
            DeleteObject(font);
        }
        EndPaint(hwnd, &paint);
    }
}

unsafe fn draw_clock_card(
    dc: HDC,
    app: &mut App,
    left: i32,
    dock_top: i32,
    width: i32,
    card_color: u32,
    text_color: u32,
) {
    unsafe {
        capsule(
            dc,
            left,
            dock_top + 12,
            left + width,
            dock_top + 54,
            card_color,
        );
        let mut now: SYSTEMTIME = zeroed();
        GetLocalTime(&mut now);
        if app.settings.date {
            const DAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
            const MONTHS: [&str; 12] = [
                "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
            ];
            let day = DAYS[now.wDayOfWeek as usize % DAYS.len()];
            let month = MONTHS[now.wMonth.saturating_sub(1) as usize % MONTHS.len()];
            styled_label(
                dc,
                left + 10,
                dock_top + 17,
                day,
                11,
                FW_SEMIBOLD,
                rgb(106, 170, 255),
            );
            styled_label(
                dc,
                left + 10,
                dock_top + 33,
                &format!("{} {month}", now.wDay),
                15,
                FW_SEMIBOLD,
                text_color,
            );
            if app.settings.calendar {
                app.hits.push(HitBox {
                    rect: rect(
                        left,
                        dock_top + 12,
                        left + if app.settings.clock { 87 } else { width },
                        dock_top + 54,
                    ),
                    action: HitAction::Calendar,
                });
            }
        }
        if app.settings.clock {
            let clock_left = if app.settings.date {
                left + 92
            } else {
                left + 12
            };
            if app.settings.date {
                let divider = CreateSolidBrush(rgb(73, 98, 142));
                FillRect(
                    dc,
                    &rect(left + 87, dock_top + 19, left + 88, dock_top + 48),
                    divider,
                );
                DeleteObject(divider);
            }
            styled_label(
                dc,
                clock_left,
                dock_top + 24,
                &format!("{:02}:{:02}", now.wHour, now.wMinute),
                18,
                FW_SEMIBOLD,
                text_color,
            );
        }
    }
}

unsafe fn styled_label(dc: HDC, x: i32, y: i32, text: &str, size: i32, weight: u32, color: u32) {
    unsafe {
        let font = CreateFontW(
            -size,
            0,
            0,
            0,
            weight as i32,
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
        if font.is_null() {
            label(dc, x, y, text);
            return;
        }
        let old_font = SelectObject(dc, font);
        let old_color = SetTextColor(dc, color);
        label(dc, x, y, text);
        SetTextColor(dc, old_color);
        SelectObject(dc, old_font);
        DeleteObject(font);
    }
}

unsafe fn draw_pin_icon(dc: HDC, target: &str, x: i32, y: i32, _label_text: &str) {
    unsafe {
        let resolved = if !target.contains('\\') && !target.contains('/') {
            let target_wide = wide(target);
            let needed = SearchPathW(
                null(),
                target_wide.as_ptr(),
                null(),
                0,
                null_mut(),
                null_mut(),
            );
            if needed > 0 && needed < 32768 {
                let mut buffer = vec![0u16; needed as usize + 1];
                let written = SearchPathW(
                    null(),
                    target_wide.as_ptr(),
                    null(),
                    buffer.len() as u32,
                    buffer.as_mut_ptr(),
                    null_mut(),
                );
                if written > 0 && written < buffer.len() as u32 {
                    String::from_utf16_lossy(&buffer[..written as usize])
                } else {
                    target.to_string()
                }
            } else {
                target.to_string()
            }
        } else {
            target.to_string()
        };
        if !draw_file_icon(dc, &resolved, x, y) {
            draw_generic_icon(dc, x, y);
        }
    }
}

unsafe fn draw_file_icon(dc: HDC, target: &str, x: i32, y: i32) -> bool {
    unsafe {
        let mut info: SHFILEINFOW = zeroed();
        let found = SHGetFileInfoW(
            wide(target).as_ptr(),
            0,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON,
        );
        if found != 0 && !info.hIcon.is_null() {
            DrawIconEx(dc, x, y, info.hIcon, 28, 28, 0, null_mut(), DI_NORMAL);
            DestroyIcon(info.hIcon);
            return true;
        }
        let found = SHGetFileInfoW(
            wide(target).as_ptr(),
            FILE_ATTRIBUTE_NORMAL,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_USEFILEATTRIBUTES,
        );
        if found != 0 && !info.hIcon.is_null() {
            DrawIconEx(dc, x, y, info.hIcon, 28, 28, 0, null_mut(), DI_NORMAL);
            DestroyIcon(info.hIcon);
            return true;
        }
        false
    }
}

unsafe fn draw_process_icon(dc: HDC, hwnd: HWND, x: i32, y: i32) {
    unsafe {
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id != 0 {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
            if !process.is_null() {
                let mut path = vec![0u16; 2048];
                let mut length = path.len() as u32;
                let found = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length);
                CloseHandle(process);
                if found != 0 {
                    let target = String::from_utf16_lossy(&path[..length as usize]);
                    if draw_file_icon(dc, &target, x, y) {
                        return;
                    }
                }
            }
        }
        draw_generic_icon(dc, x, y);
    }
}

unsafe fn draw_generic_icon(dc: HDC, x: i32, y: i32) {
    unsafe {
        let icon = LoadIconW(null_mut(), IDI_APPLICATION);
        if !icon.is_null() {
            DrawIconEx(dc, x, y, icon, 28, 28, 0, null_mut(), DI_NORMAL);
        }
    }
}

unsafe fn draw_settings_panel(
    dc: HDC,
    app: &mut App,
    width: i32,
    surface_color: u32,
    button_color: u32,
    text_color: u32,
) {
    let left = width - 560;
    let top = if app.settings.dock_top {
        DOCK_HEIGHT - 16
    } else {
        4
    };
    unsafe {
        capsule(
            dc,
            left,
            top,
            width - 8,
            top + SETTINGS_HEIGHT - 2,
            surface_color,
        );
        label(dc, left + 20, top + 20, "Pengaturan theDOCK");
        capsule(dc, width - 54, top + 12, width - 22, top + 44, button_color);
        label(dc, width - 44, top + 19, "×");
        app.hits.push(HitBox {
            rect: rect(width - 54, top + 12, width - 22, top + 44),
            action: HitAction::Settings,
        });
        let line = CreateSolidBrush(if app.settings.light_glass {
            rgb(156, 181, 216)
        } else {
            rgb(87, 120, 174)
        });
        FillRect(dc, &rect(left + 18, top + 50, width - 24, top + 51), line);
        DeleteObject(line);
        for (index, (_, setting, label_text)) in SETTINGS_MENU.iter().enumerate() {
            let column = index / 10;
            let row = index % 10;
            let x = left + 20 + column as i32 * 266;
            let y = top + 64 + row as i32 * 30;
            SetTextColor(dc, text_color);
            label(dc, x, y + 3, label_text);
            let enabled = app.settings.enabled(*setting);
            let switch_left = x + 209;
            let switch_color = if enabled {
                rgb(45, 120, 236)
            } else if app.settings.light_glass {
                rgb(168, 184, 208)
            } else {
                rgb(63, 79, 110)
            };
            capsule(dc, switch_left, y, switch_left + 38, y + 22, switch_color);
            let knob_left = if enabled {
                switch_left + 20
            } else {
                switch_left + 3
            };
            capsule(
                dc,
                knob_left,
                y + 3,
                knob_left + 15,
                y + 19,
                rgb(245, 249, 255),
            );
            app.hits.push(HitBox {
                rect: rect(x, y - 2, x + 252, y + 25),
                action: HitAction::ToggleSetting(*setting),
            });
        }
        label(
            dc,
            left + 20,
            top + 369,
            "Klik kanan dock untuk ukuran dan keluar",
        );
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

unsafe fn open_start_menu() {
    unsafe { send_key(VK_LWIN) };
}

unsafe fn send_key(key: u16) {
    let down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                ..KEYBDINPUT::default()
            },
        },
    };
    let up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: KEYEVENTF_KEYUP,
                ..KEYBDINPUT::default()
            },
        },
    };
    unsafe { SendInput(2, [down, up].as_ptr(), size_of::<INPUT>() as i32) };
}

unsafe fn capsule(dc: HDC, left: i32, top: i32, right: i32, bottom: i32, color: u32) {
    unsafe {
        if right - left < 4 || bottom - top < 4 {
            return;
        }
        let border = CreateSolidBrush(tint(color, if bottom - top > 55 { 72 } else { 29 }));
        let previous_brush = SelectObject(dc, border);
        let previous_pen = SelectObject(dc, GetStockObject(NULL_PEN));
        let radius = (bottom - top).min(34);
        RoundRect(dc, left, top, right, bottom, radius, radius);
        SelectObject(dc, previous_brush);
        SelectObject(dc, previous_pen);
        DeleteObject(border);

        let clip = CreateRoundRectRgn(left + 1, top + 1, right - 1, bottom - 1, radius, radius);
        if clip.is_null() {
            return;
        }
        let saved = SaveDC(dc);
        if saved != 0 {
            SelectClipRgn(dc, clip);
            let vertices = [
                gradient_vertex(left, top, tint(color, 14)),
                gradient_vertex(right, bottom, tint(color, -10)),
            ];
            let rectangle = GRADIENT_RECT {
                UpperLeft: 0,
                LowerRight: 1,
            };
            GradientFill(
                dc,
                vertices.as_ptr(),
                vertices.len() as u32,
                (&rectangle as *const GRADIENT_RECT).cast::<c_void>(),
                1,
                GRADIENT_FILL_RECT_V,
            );
            RestoreDC(dc, saved);
        }
        DeleteObject(clip);
    }
}

fn gradient_vertex(x: i32, y: i32, color: u32) -> TRIVERTEX {
    TRIVERTEX {
        x,
        y,
        Red: ((color & 0xff) as u16) << 8,
        Green: (((color >> 8) & 0xff) as u16) << 8,
        Blue: (((color >> 16) & 0xff) as u16) << 8,
        Alpha: 0,
    }
}

fn tint(color: u32, adjustment: i16) -> u32 {
    let channel = |shift| (((color >> shift) & 0xffu32) as i16 + adjustment).clamp(0, 255) as u8;
    rgb(channel(0), channel(8), channel(16))
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
