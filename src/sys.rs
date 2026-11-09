//! Piccoli wrapper sulle API Win32 usate dall'applicazione.

use std::ffi::{OsStr, c_void};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, MAX_PATH};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows_sys::Win32::UI::Shell::{
    BFFM_INITIALIZED, BFFM_SETSELECTIONW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
    SHBrowseForFolderW, SHGetPathFromIDListW, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONWARNING, MB_OK, MessageBoxW, SW_SHOWNORMAL, SendMessageW,
};

pub const APP_NAME: &str = "FolderTrayMenu";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Stringa UTF-16 terminata da zero.
pub fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

pub fn message_box(hwnd: HWND, text: &str) {
    let text = wide(text);
    let title = wide(APP_NAME);
    unsafe { MessageBoxW(hwnd, text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING) };
}

/// Apre un file o una cartella con l'azione predefinita della shell.
pub fn shell_open(hwnd: HWND, path: &Path) {
    let file = wide(path);
    let dir = if path.is_dir() { Some(path) } else { path.parent() };
    let dir = dir.map(wide);
    let result = unsafe {
        ShellExecuteW(
            hwnd,
            null(),
            file.as_ptr(),
            null(),
            dir.as_ref().map_or(null(), |d| d.as_ptr()),
            SW_SHOWNORMAL,
        )
    };
    // Valori <= 32 indicano un errore. L'annullamento di un prompt UAC (5) non va segnalato.
    let code = result as isize;
    if code <= 32 && code != 5 {
        message_box(hwnd, &crate::i18n::fmt(crate::i18n::t().open_failed, &[&path.display(), &code]));
    }
}

// ------------------------------------------------------------- autostart

pub fn autostart_enabled() -> bool {
    let key = wide(RUN_KEY);
    let name = wide(APP_NAME);
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, null_mut(), null_mut(), null_mut())
    };
    status == ERROR_SUCCESS
}

pub fn set_autostart(enabled: bool) {
    let key = wide(RUN_KEY);
    let name = wide(APP_NAME);
    unsafe {
        if enabled {
            let Ok(exe) = std::env::current_exe() else { return };
            let value = wide(format!("\"{}\"", exe.display()));
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                value.as_ptr() as *const c_void,
                (value.len() * 2) as u32,
            );
        } else {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr());
        }
    }
}

// ------------------------------------------------------- scelta cartella

/// Dialogo di selezione cartella; `initial` viene preselezionata.
pub fn pick_folder(hwnd: HWND, initial: &Path) -> Option<PathBuf> {
    unsafe extern "system" fn callback(hwnd: HWND, msg: u32, _lparam: LPARAM, data: LPARAM) -> i32 {
        if msg == BFFM_INITIALIZED {
            unsafe { SendMessageW(hwnd, BFFM_SETSELECTIONW, 1, data) };
        }
        0
    }

    let title = wide(crate::i18n::t().pick_folder_title);
    let initial = wide(initial);
    let mut display = [0u16; MAX_PATH as usize];

    let mut info: BROWSEINFOW = unsafe { std::mem::zeroed() };
    info.hwndOwner = hwnd;
    info.pszDisplayName = display.as_mut_ptr();
    info.lpszTitle = title.as_ptr();
    info.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
    info.lpfn = Some(callback);
    info.lParam = initial.as_ptr() as LPARAM;

    unsafe {
        let pidl = SHBrowseForFolderW(&info);
        if pidl.is_null() {
            return None;
        }
        let mut buffer = [0u16; 32768];
        let ok = SHGetPathFromIDListW(pidl, buffer.as_mut_ptr());
        CoTaskMemFree(pidl as *const c_void);
        if ok == 0 {
            return None;
        }
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        Some(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len])))
    }
}
