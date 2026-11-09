//! Finestra "Impostazioni": cartella radice e scorciatoia globale.
//!
//! Finestra Win32 semplice con un ciclo di messaggi proprio: `IsDialogMessageW`
//! fornisce Tab/Shift+Tab tra i controlli, Invio = OK ed Esc = Annulla.
//! La scorciatoia si inserisce con il controllo hotkey nativo (`msctls_hotkey32`):
//! basta premere la combinazione desiderata.
//!
//! La window procedure si limita a registrare i comandi; le operazioni che
//! aprono finestre modali o toccano lo stato dell'app (Sfoglia, OK) vengono
//! eseguite dal ciclo di messaggi, fuori dalla window procedure, per evitare
//! rientri mentre lo stato è in prestito.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_BTNFACE, CreateFontIndirectW, DeleteObject, GetMonitorInfoW, HBRUSH, HFONT, MONITOR_DEFAULTTONEAREST,
    MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{ICC_HOTKEY_CLASS, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx};
use windows_sys::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI, SystemParametersInfoForDpi,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBS_DROPDOWNLIST, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, ES_AUTOHSCROLL,
    GetCursorPos, GetMessageW, GetWindowTextLengthW, GetWindowTextW, HMENU, IDC_ARROW, IDCANCEL, IDOK, IsDialogMessageW,
    LoadCursorW, MSG, NONCLIENTMETRICSW, PostQuitMessage, RegisterClassW, SPI_GETNONCLIENTMETRICS, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetWindowPos, SetWindowTextW, ShowWindow,
    TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_DPICHANGED, WM_SETFONT, WM_USER, WNDCLASSW,
    WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

use crate::i18n::{self, ALL as LANGUAGES};
use crate::settings::{format_hotkey, is_extended_key, parse_hotkey};
use crate::sys::{self, wide};

// Messaggi e flag del controllo hotkey.
const HKM_SETHOTKEY: u32 = WM_USER + 1;
const HKM_GETHOTKEY: u32 = WM_USER + 2;
const HKM_SETRULES: u32 = WM_USER + 3;
const HOTKEYF_SHIFT: u32 = 0x01;
const HOTKEYF_CONTROL: u32 = 0x02;
const HOTKEYF_ALT: u32 = 0x04;
const HOTKEYF_EXT: u32 = 0x08;
const HKCOMB_NONE: usize = 0x01;
const HKCOMB_S: usize = 0x02;

const ID_FOLDER: i32 = 100;
const ID_BROWSE: i32 = 101;
const ID_HOTKEY: i32 = 102;
const ID_CLEAR: i32 = 103;
const ID_LANGUAGE: i32 = 104;

/// Area client in pixel logici (96 DPI).
const CLIENT_WIDTH: i32 = 440;
const CLIENT_HEIGHT: i32 = 284;
const WINDOW_STYLE_MAIN: WINDOW_STYLE = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;

/// Valori mostrati e restituiti dal dialog.
pub struct Choice {
    pub folder: PathBuf,
    pub hotkey: String,
    /// Codice lingua; vuoto = lingua di Windows.
    pub language: String,
}

#[derive(Clone, Copy)]
enum Command {
    Browse,
    Accept,
}

struct Dialog {
    hwnd: HWND,
    folder_edit: HWND,
    hotkey_ctrl: HWND,
    language_combo: HWND,
    /// Controlli figli con posizione in pixel logici: x, y, larghezza, altezza.
    layout: Vec<(HWND, [i32; 4])>,
    font: HFONT,
    /// Valore iniziale del controllo hotkey e testo originale: se l'utente non tocca
    /// il campo si conserva il testo originale (che può contenere Win, non rappresentabile).
    initial_value: usize,
    initial_hotkey: String,
    pending: Option<Command>,
    done: bool,
}

thread_local! {
    static DIALOG: RefCell<Option<Dialog>> = const { RefCell::new(None) };
}

/// Mostra il dialog e ritorna alla chiusura. `apply` viene chiamata alla pressione
/// di OK con i valori scelti: se restituisce un errore questo viene mostrato e il
/// dialog resta aperto.
pub fn show(current: &Choice, apply: impl Fn(Choice) -> Result<(), String>) {
    // Già aperto (es. richiesto di nuovo dal menu della tray): lo si porta in primo piano.
    let existing = DIALOG.with(|d| d.try_borrow().ok().and_then(|d| d.as_ref().map(|d| d.hwnd)));
    if let Some(hwnd) = existing {
        unsafe { SetForegroundWindow(hwnd) };
        return;
    }
    if DIALOG.with(|d| d.try_borrow().is_err()) {
        return;
    }

    let Some(hwnd) = create(&current.folder, &current.hotkey, &current.language) else { return };

    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        loop {
            if DIALOG.with(|d| d.borrow().as_ref().is_none_or(|d| d.done)) {
                break;
            }
            let status = GetMessageW(&mut msg, null_mut(), 0, 0);
            if status <= 0 {
                // WM_QUIT (es. "Esci" dal menu mentre il dialog è aperto): va ripassato al ciclo principale.
                if status == 0 {
                    PostQuitMessage(msg.wParam as i32);
                }
                break;
            }
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let command = DIALOG.with(|d| d.borrow_mut().as_mut().and_then(|d| d.pending.take()));
            match command {
                Some(Command::Browse) => browse(hwnd),
                Some(Command::Accept) => accept(hwnd, &apply),
                None => {}
            }
        }

        if let Some(dialog) = DIALOG.with(|d| d.borrow_mut().take()) {
            DestroyWindow(dialog.hwnd);
            DeleteObject(dialog.font);
        }
    }
}

fn create(folder: &Path, hotkey: &str, language: &str) -> Option<HWND> {
    let t = i18n::t();
    unsafe {
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_HOTKEY_CLASS | ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&controls);

        let instance = GetModuleHandleW(null());
        let class = wide("FolderTrayMenuSettings");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = instance;
        wc.lpszClassName = class.as_ptr();
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        wc.hbrBackground = (COLOR_BTNFACE + 1) as usize as HBRUSH;
        RegisterClassW(&wc); // fallisce senza danni se la classe è già registrata

        // Posizione: centrata sull'area di lavoro del monitor in cui si trova il mouse.
        let mut cursor = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor);
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(monitor, &mut info);
        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);

        let (width, height) = window_size(dpi_x);
        let work = info.rcWork;
        let x = work.left + ((work.right - work.left) - width) / 2;
        let y = work.top + ((work.bottom - work.top) - height) / 2;

        let title = wide(t.settings_title);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WINDOW_STYLE_MAIN,
            x,
            y,
            width,
            height,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            return None;
        }

        let static_style = WS_CHILD | WS_VISIBLE;
        let button_style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32;

        let mut layout = Vec::new();
        let mut add = |class: &str, text: &str, style: WINDOW_STYLE, ex: WINDOW_EX_STYLE, id: i32, rect: [i32; 4]| {
            let class = wide(class);
            let text = wide(text);
            let child = CreateWindowExW(
                ex,
                class.as_ptr(),
                text.as_ptr(),
                style,
                0,
                0,
                0,
                0,
                hwnd,
                id as usize as HMENU,
                instance,
                null(),
            );
            layout.push((child, rect));
            child
        };

        // L'ordine di creazione è l'ordine di Tab.
        add("STATIC", t.root_folder_label, static_style, 0, -1, [16, 14, 408, 18]);
        let folder_edit = add(
            "EDIT",
            &folder.display().to_string(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            WS_EX_CLIENTEDGE,
            ID_FOLDER,
            [16, 34, 316, 24],
        );
        add("BUTTON", t.browse, button_style, 0, ID_BROWSE, [340, 33, 84, 26]);

        add("STATIC", t.hotkey_label, static_style, 0, -1, [16, 74, 408, 18]);
        let hotkey_ctrl = add(
            "msctls_hotkey32",
            "",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            WS_EX_CLIENTEDGE,
            ID_HOTKEY,
            [16, 94, 316, 24],
        );
        add("BUTTON", t.hotkey_clear, button_style, 0, ID_CLEAR, [340, 93, 84, 26]);

        let parsed = parse_hotkey(hotkey);
        let hint = if parsed.is_some_and(|(mods, _)| mods & MOD_WIN != 0) {
            i18n::fmt(t.hotkey_hint_win, &[&hotkey])
        } else {
            t.hotkey_hint.to_string()
        };
        add("STATIC", &hint, static_style, 0, -1, [16, 124, 408, 48]);

        add("STATIC", t.language_label, static_style, 0, -1, [16, 182, 408, 18]);
        // Per una combo a discesa l'altezza include l'elenco aperto.
        let language_combo = add(
            "COMBOBOX",
            "",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            0,
            ID_LANGUAGE,
            [16, 202, 240, 240],
        );

        add(
            "BUTTON",
            t.ok,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
            0,
            IDOK,
            [256, 244, 80, 28],
        );
        add("BUTTON", t.cancel, button_style, 0, IDCANCEL, [344, 244, 80, 28]);

        // Elenco lingue: "Automatica" e poi ogni lingua col proprio nome.
        let auto = wide(t.language_auto);
        SendMessageW(language_combo, CB_ADDSTRING, 0, auto.as_ptr() as LPARAM);
        for lang in LANGUAGES {
            let name = wide(lang.native_name());
            SendMessageW(language_combo, CB_ADDSTRING, 0, name.as_ptr() as LPARAM);
        }
        let selected = i18n::Lang::from_code(language)
            .and_then(|l| LANGUAGES.iter().position(|&x| x == l))
            .map_or(0, |i| i + 1);
        SendMessageW(language_combo, CB_SETCURSEL, selected, 0);

        // Valore iniziale del controllo hotkey; senza modificatori il controllo forza Ctrl+Alt.
        SendMessageW(hotkey_ctrl, HKM_SETRULES, HKCOMB_NONE | HKCOMB_S, (HOTKEYF_CONTROL | HOTKEYF_ALT) as LPARAM);
        let initial_value = parsed.map_or(0, |(mods, vk)| control_value(mods, vk));
        SendMessageW(hotkey_ctrl, HKM_SETHOTKEY, initial_value, 0);

        let dialog = Dialog {
            hwnd,
            folder_edit,
            hotkey_ctrl,
            language_combo,
            layout,
            font: null_mut(),
            initial_value,
            initial_hotkey: hotkey.to_string(),
            pending: None,
            done: false,
        };
        DIALOG.with(|d| *d.borrow_mut() = Some(dialog));
        apply_layout(GetDpiForWindow(hwnd));

        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(folder_edit);
        Some(hwnd)
    }
}

fn window_size(dpi: u32) -> (i32, i32) {
    let mut rect = RECT { left: 0, top: 0, right: scale(CLIENT_WIDTH, dpi), bottom: scale(CLIENT_HEIGHT, dpi) };
    unsafe { AdjustWindowRectExForDpi(&mut rect, WINDOW_STYLE_MAIN, 0, 0, dpi) };
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn scale(value: i32, dpi: u32) -> i32 {
    value * dpi as i32 / 96
}

/// Posiziona i controlli e imposta il font di sistema per il DPI indicato.
fn apply_layout(dpi: u32) {
    let font = unsafe {
        let mut metrics: NONCLIENTMETRICSW = std::mem::zeroed();
        metrics.cbSize = size_of::<NONCLIENTMETRICSW>() as u32;
        SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            &mut metrics as *mut _ as *mut _,
            0,
            dpi,
        );
        CreateFontIndirectW(&metrics.lfMessageFont)
    };

    let old_font = DIALOG.with(|d| {
        let mut d = d.borrow_mut();
        let dialog = d.as_mut()?;
        for &(child, [x, y, w, h]) in &dialog.layout {
            unsafe {
                SetWindowPos(
                    child,
                    null_mut(),
                    scale(x, dpi),
                    scale(y, dpi),
                    scale(w, dpi),
                    scale(h, dpi),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                SendMessageW(child, WM_SETFONT, font as WPARAM, 1);
            }
        }
        Some(std::mem::replace(&mut dialog.font, font))
    });

    match old_font {
        Some(old) if !old.is_null() => unsafe {
            DeleteObject(old);
        },
        Some(_) => {}
        None => unsafe {
            DeleteObject(font);
        },
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let command = match id {
                IDOK => Some(Command::Accept),
                ID_BROWSE => Some(Command::Browse),
                IDCANCEL => {
                    close();
                    None
                }
                ID_CLEAR => {
                    with_dialog(|d| unsafe {
                        SendMessageW(d.hotkey_ctrl, HKM_SETHOTKEY, 0, 0);
                        SetFocus(d.hotkey_ctrl);
                    });
                    None
                }
                _ => None,
            };
            if let Some(command) = command {
                with_dialog(|d| d.pending = Some(command));
            }
            0
        }
        WM_CLOSE => {
            close();
            0
        }
        WM_DPICHANGED => {
            // Spostata su un monitor con DPI diverso: si usa il rettangolo suggerito e si riscala.
            let suggested = unsafe { &*(lparam as *const RECT) };
            unsafe {
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            apply_layout((wparam & 0xFFFF) as u32);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn with_dialog(f: impl FnOnce(&mut Dialog)) {
    DIALOG.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut()
            && let Some(dialog) = d.as_mut()
        {
            f(dialog);
        }
    });
}

fn close() {
    with_dialog(|d| d.done = true);
}

fn browse(hwnd: HWND) {
    let Some(edit) = DIALOG.with(|d| d.borrow().as_ref().map(|d| d.folder_edit)) else { return };
    let current = PathBuf::from(window_text(edit));
    if let Some(folder) = sys::pick_folder(hwnd, &current) {
        let text = wide(&folder);
        unsafe { SetWindowTextW(edit, text.as_ptr()) };
    }
}

fn accept(hwnd: HWND, apply: &impl Fn(Choice) -> Result<(), String>) {
    let t = i18n::t();
    let Some((edit, hotkey_ctrl, combo, initial_value, initial_hotkey)) = DIALOG.with(|d| {
        d.borrow()
            .as_ref()
            .map(|d| (d.folder_edit, d.hotkey_ctrl, d.language_combo, d.initial_value, d.initial_hotkey.clone()))
    }) else {
        return;
    };

    let folder_text = window_text(edit);
    let folder_text = folder_text.trim().trim_matches('"');
    if folder_text.is_empty() {
        sys::message_box(hwnd, t.folder_required);
        unsafe { SetFocus(edit) };
        return;
    }
    let folder = std::path::absolute(folder_text).unwrap_or_else(|_| PathBuf::from(folder_text));
    if !folder.is_dir() {
        sys::message_box(hwnd, &i18n::fmt(t.folder_missing, &[&folder.display()]));
        unsafe { SetFocus(edit) };
        return;
    }

    let value = unsafe { SendMessageW(hotkey_ctrl, HKM_GETHOTKEY, 0, 0) } as usize & 0xFFFF;
    let hotkey = if value == initial_value {
        initial_hotkey
    } else if value == 0 {
        String::new()
    } else {
        match hotkey_text(value) {
            Some(text) => text,
            None => {
                sys::message_box(hwnd, t.key_unsupported);
                unsafe { SetFocus(hotkey_ctrl) };
                return;
            }
        }
    };

    let selected = unsafe { SendMessageW(combo, CB_GETCURSEL, 0, 0) };
    let language = usize::try_from(selected)
        .ok()
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| LANGUAGES.get(i))
        .map_or(String::new(), |l| l.code().to_string());

    match apply(Choice { folder, hotkey, language }) {
        Ok(()) => close(),
        Err(message) => sys::message_box(hwnd, &message),
    }
}

/// Valore per HKM_SETHOTKEY: virtual key nel byte basso, flag HOTKEYF_* nel byte alto.
fn control_value(mods: HOT_KEY_MODIFIERS, vk: u32) -> usize {
    let mut flags = 0;
    if mods & MOD_SHIFT != 0 {
        flags |= HOTKEYF_SHIFT;
    }
    if mods & MOD_CONTROL != 0 {
        flags |= HOTKEYF_CONTROL;
    }
    if mods & MOD_ALT != 0 {
        flags |= HOTKEYF_ALT;
    }
    if is_extended_key(vk) {
        flags |= HOTKEYF_EXT;
    }
    ((flags << 8) | (vk & 0xFF)) as usize
}

fn hotkey_text(value: usize) -> Option<String> {
    let vk = (value & 0xFF) as u32;
    let flags = ((value >> 8) & 0xFF) as u32;
    let mut mods: HOT_KEY_MODIFIERS = 0;
    if flags & HOTKEYF_CONTROL != 0 {
        mods |= MOD_CONTROL;
    }
    if flags & HOTKEYF_ALT != 0 {
        mods |= MOD_ALT;
    }
    if flags & HOTKEYF_SHIFT != 0 {
        mods |= MOD_SHIFT;
    }
    format_hotkey(mods, vk)
}

fn window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut buffer = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
    }
}
