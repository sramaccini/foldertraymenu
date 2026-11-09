//! Costruzione del menu a partire dal file system.
//!
//! Si usano i menu popup nativi di Windows, che offrono già la navigazione da
//! tastiera (frecce, Invio, Esc, ← e → tra i sottomenu). In più `on_menu_char`
//! aggiunge la ricerca per lettera iniziale.
//!
//! Le sottocartelle vengono lette solo quando il loro sottomenu sta per aprirsi
//! (`WM_INITMENUPOPUP`), quindi il menu riflette sempre lo stato del disco.

use std::collections::HashMap;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::LRESULT;
use windows_sys::Win32::Graphics::Gdi::HBITMAP;
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};
use windows_sys::Win32::UI::Shell::StrCmpLogicalW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DeleteMenu, GetMenuItemCount, GetMenuState, GetMenuStringW, HMENU,
    MENUITEMINFOW, MF_BYPOSITION, MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_HILITE, MF_POPUP,
    MF_SEPARATOR, MF_STRING, MIIM_BITMAP, MNC_SELECT, SetMenuItemInfoW,
};

use crate::icons::IconCache;
use crate::settings::Settings;
use crate::sys::wide;

/// Estensioni nascoste nel nome mostrato, come fa Esplora file.
const HIDDEN_EXTENSIONS: &[&str] = &["lnk", "url", "appref-ms"];

#[derive(Clone)]
pub enum Action {
    Open(PathBuf),
    OpenRoot,
    OpenSettings,
    ToggleHidden,
    ToggleAutostart,
    Exit,
}

/// Stato valido per una singola apertura del menu.
pub struct MenuBuilder {
    actions: Vec<Action>,
    /// Sottomenu non ancora popolati → cartella che rappresentano.
    pending: HashMap<usize, PathBuf>,
    icons: IconCache,
}

struct Entry {
    path: PathBuf,
    /// Nome mostrato, UTF-16 terminato da zero (serve anche per l'ordinamento).
    label: Vec<u16>,
    is_dir: bool,
}

impl MenuBuilder {
    pub fn new() -> Self {
        Self { actions: Vec::new(), pending: HashMap::new(), icons: IconCache::new() }
    }

    /// Azione associata all'id restituito da `TrackPopupMenu` (0 = annullato).
    pub fn action(&self, id: u32) -> Option<Action> {
        id.checked_sub(1).and_then(|i| self.actions.get(i as usize)).cloned()
    }

    /// Crea il menu principale. Va distrutto con `DestroyMenu` dopo l'uso.
    pub fn build_root(&mut self, settings: &Settings, hotkey_label: Option<&str>, autostart: bool) -> HMENU {
        self.actions.clear();
        self.pending.clear();

        unsafe {
            let root = CreatePopupMenu();
            if settings.root_folder.is_dir() {
                self.add_folder_entries(root, &settings.root_folder, settings, false);
            } else {
                append_disabled(root, &format!("Cartella non trovata: {}", settings.root_folder.display()));
            }
            AppendMenuW(root, MF_SEPARATOR, 0, std::ptr::null());

            let options = CreatePopupMenu();
            self.append_action(options, "&Apri cartella radice", Action::OpenRoot, false);
            self.append_action(options, "&Impostazioni…", Action::OpenSettings, false);
            self.append_action(options, "Mostra elementi &nascosti", Action::ToggleHidden, settings.show_hidden);
            self.append_action(options, "Avvia con &Windows", Action::ToggleAutostart, autostart);
            AppendMenuW(options, MF_SEPARATOR, 0, std::ptr::null());
            append_disabled(options, &match hotkey_label {
                Some(label) => format!("Scorciatoia: {label}"),
                None => "Scorciatoia: non attiva".to_string(),
            });
            AppendMenuW(options, MF_SEPARATOR, 0, std::ptr::null());
            self.append_action(options, "&Esci", Action::Exit, false);

            let label = wide("FolderTrayMenu");
            AppendMenuW(root, MF_POPUP | MF_STRING, options as usize, label.as_ptr());
            root
        }
    }

    /// Da chiamare su `WM_INITMENUPOPUP`: popola il sottomenu di una cartella alla prima apertura.
    pub fn on_init_popup(&mut self, menu: HMENU, settings: &Settings) {
        // remove(): ogni sottomenu viene letto una sola volta per apertura del menu.
        let Some(folder) = self.pending.remove(&(menu as usize)) else { return };

        unsafe {
            while GetMenuItemCount(menu) > 0 {
                DeleteMenu(menu, 0, MF_BYPOSITION);
            }
        }
        self.add_folder_entries(menu, &folder, settings, true);
    }

    fn add_folder_entries(&mut self, menu: HMENU, folder: &Path, settings: &Settings, is_submenu: bool) {
        if is_submenu {
            // Unico modo, con i menu nativi, per aprire la cartella stessa anche da tastiera.
            self.append_action(menu, "Apri in Esplora file", Action::Open(folder.to_path_buf()), false);
            unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null()) };
        }

        let entries = match read_entries(folder, settings.show_hidden) {
            Ok(entries) => entries,
            Err(err) => {
                append_disabled(menu, &format!("(impossibile leggere: {err})"));
                return;
            }
        };

        if entries.is_empty() {
            append_disabled(menu, "(vuota)");
            return;
        }

        for entry in entries {
            let bitmap = self.icons.menu_bitmap(&entry.path, entry.is_dir);
            unsafe {
                if entry.is_dir {
                    let sub = CreatePopupMenu();
                    // Segnaposto: mostra la freccia del sottomenu senza leggere subito il disco.
                    append_disabled(sub, "…");
                    self.pending.insert(sub as usize, entry.path);
                    AppendMenuW(menu, MF_POPUP | MF_STRING, sub as usize, entry.label.as_ptr());
                } else {
                    let id = self.push(Action::Open(entry.path));
                    AppendMenuW(menu, MF_STRING, id, entry.label.as_ptr());
                }
                set_last_item_bitmap(menu, bitmap);
            }
        }
    }

    fn push(&mut self, action: Action) -> usize {
        self.actions.push(action);
        self.actions.len()
    }

    fn append_action(&mut self, menu: HMENU, label: &str, action: Action, checked: bool) {
        let id = self.push(action);
        let label = wide(label);
        let flags = MF_STRING | if checked { MF_CHECKED } else { 0 };
        unsafe { AppendMenuW(menu, flags, id, label.as_ptr()) };
    }
}

fn read_entries(folder: &Path, show_hidden: bool) -> std::io::Result<Vec<Entry>> {
    let mut entries = Vec::new();

    for item in std::fs::read_dir(folder)? {
        let Ok(item) = item else { continue };
        let path = item.path();
        // fs::metadata segue link simbolici e junction verso le cartelle.
        let Ok(meta) = std::fs::metadata(&path).or_else(|_| item.metadata()) else { continue };

        if !show_hidden && meta.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0 {
            continue;
        }
        let name = item.file_name().to_string_lossy().into_owned();
        if name.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }

        let is_dir = meta.is_dir();
        let shown = match path.extension().map(|e| e.to_string_lossy().to_lowercase()) {
            Some(ext) if !is_dir && HIDDEN_EXTENSIONS.contains(&ext.as_str()) => {
                path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(name)
            }
            _ => name,
        };

        // "&" verrebbe interpretato come acceleratore: va raddoppiato.
        entries.push(Entry { path, label: wide(shown.replace('&', "&&")), is_dir });
    }

    // Prima le cartelle, poi i file; ordinamento "naturale" come Esplora file (file2 < file10).
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| unsafe { StrCmpLogicalW(a.label.as_ptr(), b.label.as_ptr()) }.cmp(&0))
    });
    Ok(entries)
}

fn append_disabled(menu: HMENU, text: &str) {
    let text = wide(text);
    unsafe { AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, text.as_ptr()) };
}

unsafe fn set_last_item_bitmap(menu: HMENU, bitmap: HBITMAP) {
    if bitmap.is_null() {
        return;
    }
    unsafe {
        let position = GetMenuItemCount(menu) - 1;
        let mut info: MENUITEMINFOW = std::mem::zeroed();
        info.cbSize = size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_BITMAP;
        info.hbmpItem = bitmap;
        SetMenuItemInfoW(menu, position as u32, 1, &info);
    }
}

/// Gestione di `WM_MENUCHAR`: premendo una lettera si passa alla voce successiva
/// che inizia con quella lettera (ripetendola si scorre tra le corrispondenze).
pub fn on_menu_char(menu: HMENU, ch: u16) -> LRESULT {
    let Some(target) = char::from_u32(ch as u32).map(|c| c.to_lowercase().collect::<String>()) else {
        return 0;
    };

    unsafe {
        let count = GetMenuItemCount(menu);
        if count <= 0 {
            return 0;
        }

        let current = (0..count).find(|&i| GetMenuState(menu, i as u32, MF_BYPOSITION) & MF_HILITE != 0);
        let start = current.map_or(0, |i| i + 1);

        for offset in 0..count {
            let index = (start + offset) % count;
            let state = GetMenuState(menu, index as u32, MF_BYPOSITION);
            if state & (MF_SEPARATOR | MF_DISABLED | MF_GRAYED) != 0 {
                continue;
            }

            let mut buffer = [0u16; 260];
            let len = GetMenuStringW(menu, index as u32, buffer.as_mut_ptr(), buffer.len() as i32, MF_BYPOSITION);
            if len <= 0 {
                continue;
            }
            let text = String::from_utf16_lossy(&buffer[..len as usize]);
            let first = text.trim_start_matches('&').chars().next().map(|c| c.to_lowercase().collect::<String>());
            if first.as_deref() == Some(target.as_str()) {
                return ((MNC_SELECT as LRESULT) << 16) | index as LRESULT;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;

    fn labels(entries: &[Entry]) -> Vec<String> {
        entries.iter().map(|e| String::from_utf16_lossy(&e.label[..e.label.len() - 1])).collect()
    }

    #[test]
    fn cartelle_prima_ordinamento_naturale_filtri() {
        let root = std::env::temp_dir().join(format!("ftm-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Beta & co")).unwrap();
        std::fs::create_dir_all(root.join("alfa")).unwrap();
        for f in ["file10.txt", "file2.txt", "Zeta.md", "Blocco note.lnk", "desktop.ini", "nascosto.txt"] {
            std::fs::write(root.join(f), b"").unwrap();
        }
        // CREATE_NO_WINDOW: niente console durante i test.
        std::process::Command::new("attrib")
            .arg("+h")
            .arg(root.join("nascosto.txt"))
            .creation_flags(0x0800_0000)
            .status()
            .unwrap();

        let visible = labels(&read_entries(&root, false).unwrap());
        assert_eq!(visible, ["alfa", "Beta && co", "Blocco note", "file2.txt", "file10.txt", "Zeta.md"]);

        let all = labels(&read_entries(&root, true).unwrap());
        assert!(all.contains(&"nascosto.txt".to_string()));
        assert!(!all.contains(&"desktop.ini".to_string()));

        let _ = std::fs::remove_dir_all(&root);
    }
}
