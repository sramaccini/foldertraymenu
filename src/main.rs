//! FolderTrayMenu: icona nella system tray che mostra il contenuto di una
//! cartella come menu navigabile con mouse e tastiera.

#![windows_subsystem = "windows"]

mod i18n;
mod icons;
mod menu;
mod settings;
mod settings_dialog;
mod sys;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_DOWN};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, HICON, IDI_APPLICATION, LoadIconW, MSG, PostMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenuEx, TranslateMessage, WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_HOTKEY, WM_INITMENUPOPUP,
    WM_KEYDOWN, WM_MENUCHAR, WM_NULL, WNDCLASSW, WS_OVERLAPPED,
};

use menu::{Action, MenuBuilder};
use settings::Settings;
use sys::wide;

const WM_TRAY: u32 = WM_APP + 1;
const TRAY_ID: u32 = 1;
const HOTKEY_ID: i32 = 1;
/// NIN_SELECT | NINF_KEY: selezione dell'icona da tastiera (non esportata da windows-sys).
const NIN_KEYSELECT: u32 = NIN_SELECT | 0x1;

struct App {
    hwnd: HWND,
    settings: Settings,
    menu: MenuBuilder,
    tray_icon: HICON,
    owns_tray_icon: bool,
    hotkey_active: bool,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    static MENU_OPEN: Cell<bool> = const { Cell::new(false) };
    static LAST_OPEN_TICK: Cell<u32> = const { Cell::new(0) };
}

/// Accesso allo stato. Restituisce `None` se lo stato è già in uso: la shell può
/// rientrare nella window procedure (es. durante l'estrazione delle icone).
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|app| app.try_borrow_mut().ok().and_then(|mut app| app.as_mut().map(f)))
}

fn main() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // Istanza singola per utente (il mutex resta aperto fino alla fine del processo).
        let mutex_name = wide(r"Local\FolderTrayMenu.SingleInstance");
        CreateMutexW(null(), 1, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }

        CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32);
    }

    let mut settings = Settings::load();
    i18n::set_language(&settings.language);
    if let Some(arg) = std::env::args_os().nth(1).map(PathBuf::from).filter(|p| p.is_dir()) {
        settings.root_folder = std::path::absolute(&arg).unwrap_or(arg);
        settings.save();
    }
    let _ = std::fs::create_dir_all(&settings.root_folder);

    let hwnd = create_window();
    unsafe {
        let name = wide("TaskbarCreated");
        TASKBAR_CREATED.set(RegisterWindowMessageW(name.as_ptr()));
    }

    APP.with(|app| {
        *app.borrow_mut() = Some(App {
            hwnd,
            settings,
            menu: MenuBuilder::new(),
            tray_icon: null_mut(),
            owns_tray_icon: false,
            hotkey_active: false,
        })
    });
    with_app(|app| {
        app.load_tray_icon();
        app.register_hotkey();
        app.add_tray();
    });

    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    APP.with(|app| app.borrow_mut().take());
    unsafe { CoUninitialize() };
}

fn create_window() -> HWND {
    unsafe {
        let instance = GetModuleHandleW(null());
        let class = wide("FolderTrayMenuWindow");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = instance;
        wc.lpszClassName = class.as_ptr();
        RegisterClassW(&wc);

        // Finestra nascosta (non message-only: deve ricevere il broadcast "TaskbarCreated").
        CreateWindowExW(0, class.as_ptr(), class.as_ptr(), WS_OVERLAPPED, 0, 0, 0, 0, null_mut(), null_mut(), instance, null())
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            // NOTIFYICON_VERSION_4: evento in LOWORD(lParam), punto di ancoraggio in wParam.
            let anchor = POINT { x: (wparam & 0xFFFF) as i16 as i32, y: ((wparam >> 16) & 0xFFFF) as i16 as i32 };
            match (lparam & 0xFFFF) as u32 {
                NIN_SELECT | WM_CONTEXTMENU => show_menu(hwnd, anchor, false, true),
                NIN_KEYSELECT => show_menu(hwnd, anchor, true, true),
                _ => {}
            }
            0
        }
        WM_HOTKEY => {
            let mut cursor = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut cursor) };
            show_menu(hwnd, cursor, true, false);
            0
        }
        WM_INITMENUPOPUP => {
            with_app(|app| app.menu.on_init_popup(wparam as _, &app.settings));
            0
        }
        WM_MENUCHAR => menu::on_menu_char(lparam as _, (wparam & 0xFFFF) as u16),
        WM_DESTROY => {
            with_app(|app| app.shutdown());
            unsafe { PostQuitMessage(0) };
            0
        }
        // Explorer riavviato: l'icona va aggiunta di nuovo.
        m if m != 0 && m == TASKBAR_CREATED.get() => {
            with_app(|app| app.add_tray());
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn show_menu(hwnd: HWND, at: POINT, by_keyboard: bool, from_tray: bool) {
    if MENU_OPEN.get() {
        return;
    }
    // Invio sull'icona della tray può generare NIN_KEYSELECT due volte.
    let now = unsafe { GetTickCount() };
    if by_keyboard && now.wrapping_sub(LAST_OPEN_TICK.get()) < 400 {
        return;
    }
    LAST_OPEN_TICK.set(now);

    let autostart = sys::autostart_enabled();
    let Some(root) = with_app(|app| {
        let hotkey = app.hotkey_active.then(|| app.settings.hotkey.clone());
        app.menu.build_root(&app.settings, hotkey.as_deref(), autostart)
    }) else {
        return;
    };

    MENU_OPEN.set(true);
    let command = unsafe {
        // Senza foreground il menu non si chiuderebbe cliccando altrove.
        SetForegroundWindow(hwnd);
        if by_keyboard {
            // Evidenzia subito la prima voce, così si naviga solo con le frecce.
            PostMessageW(hwnd, WM_KEYDOWN, VK_DOWN as WPARAM, 0);
        }
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | if from_tray { TPM_BOTTOMALIGN } else { 0 };
        let command = TrackPopupMenuEx(root, flags, at.x, at.y, hwnd, null());
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(root);
        command as u32
    };
    MENU_OPEN.set(false);

    if let Some(action) = with_app(|app| app.menu.action(command)).flatten() {
        execute(hwnd, action);
    }
}

fn execute(hwnd: HWND, action: Action) {
    match action {
        Action::Open(path) => sys::shell_open(hwnd, &path),
        Action::OpenRoot => {
            if let Some(root) = with_app(|app| app.settings.root_folder.clone()) {
                sys::shell_open(hwnd, &root);
            }
        }
        Action::OpenSettings => {
            let Some(current) = with_app(|app| settings_dialog::Choice {
                folder: app.settings.root_folder.clone(),
                hotkey: app.settings.hotkey.clone(),
                language: app.settings.language.clone(),
            }) else {
                return;
            };
            // Il dialog ha un proprio ciclo di messaggi: lo stato non deve essere in prestito mentre è aperto.
            settings_dialog::show(&current, |choice| {
                with_app(|app| app.apply(choice)).unwrap_or_else(|| Err(i18n::t().apply_busy.into()))
            });
        }
        Action::ToggleHidden => {
            with_app(|app| {
                app.settings.show_hidden = !app.settings.show_hidden;
                app.settings.save();
            });
        }
        Action::ToggleAutostart => sys::set_autostart(!sys::autostart_enabled()),
        Action::Exit => unsafe {
            DestroyWindow(hwnd);
        },
    }
}

impl App {
    fn load_tray_icon(&mut self) {
        let icon = icons::shell_icon(&self.settings.root_folder, true);
        let (icon, owned) = if icon.is_null() {
            (unsafe { LoadIconW(null_mut(), IDI_APPLICATION) }, false)
        } else {
            (icon, true)
        };

        let previous = std::mem::replace(&mut self.tray_icon, icon);
        if self.owns_tray_icon && !previous.is_null() {
            unsafe { DestroyIcon(previous) };
        }
        self.owns_tray_icon = owned;
    }

    fn register_hotkey(&mut self) {
        self.hotkey_active = settings::parse_hotkey(&self.settings.hotkey)
            .is_some_and(|(mods, vk)| unsafe { RegisterHotKey(self.hwnd, HOTKEY_ID, mods | MOD_NOREPEAT, vk) } != 0);
    }

    /// Applica le impostazioni scelte nel dialog. In caso di errore nulla viene modificato.
    fn apply(&mut self, choice: settings_dialog::Choice) -> Result<(), String> {
        let settings_dialog::Choice { folder, hotkey, language } = choice;
        if !folder.is_dir() {
            return Err(i18n::fmt(i18n::t().folder_missing, &[&folder.display()]));
        }

        if hotkey != self.settings.hotkey || !self.hotkey_active {
            let parsed = if hotkey.is_empty() {
                None
            } else {
                Some(settings::parse_hotkey(&hotkey).ok_or_else(|| i18n::fmt(i18n::t().hotkey_invalid, &[&hotkey]))?)
            };

            if self.hotkey_active {
                unsafe { UnregisterHotKey(self.hwnd, HOTKEY_ID) };
                self.hotkey_active = false;
            }
            if let Some((mods, vk)) = parsed {
                if unsafe { RegisterHotKey(self.hwnd, HOTKEY_ID, mods | MOD_NOREPEAT, vk) } == 0 {
                    // Si ripristina la scorciatoia precedente.
                    self.register_hotkey();
                    return Err(i18n::fmt(i18n::t().hotkey_in_use, &[&hotkey]));
                }
                self.hotkey_active = true;
            }
        }

        let folder_changed = folder != self.settings.root_folder;
        self.settings.root_folder = folder;
        self.settings.hotkey = hotkey;
        i18n::set_language(&language);
        self.settings.language = language;
        self.settings.save();
        if folder_changed {
            self.load_tray_icon();
        }
        self.update_tray();
        Ok(())
    }

    fn notify_data(&self) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = self.hwnd;
        data.uID = TRAY_ID;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = WM_TRAY;
        data.hIcon = self.tray_icon;

        let mut tip = format!("FolderTrayMenu – {}", self.settings.root_folder.display());
        if self.hotkey_active {
            tip.push_str(&format!("\n{}", self.settings.hotkey));
        }
        let tip: Vec<u16> = tip.encode_utf16().take(data.szTip.len() - 1).collect();
        data.szTip[..tip.len()].copy_from_slice(&tip);
        data
    }

    fn add_tray(&self) {
        let mut data = self.notify_data();
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &data);
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            Shell_NotifyIconW(NIM_SETVERSION, &data);
        }
    }

    fn update_tray(&self) {
        let data = self.notify_data();
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
    }

    fn shutdown(&mut self) {
        let data = self.notify_data();
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &data);
            if self.hotkey_active {
                UnregisterHotKey(self.hwnd, HOTKEY_ID);
            }
            if self.owns_tray_icon {
                DestroyIcon(self.tray_icon);
                self.owns_tray_icon = false;
            }
        }
    }
}
