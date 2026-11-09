//! Impostazioni persistite in `%APPDATA%\FolderTrayMenu\settings.json`
//! (stesso file e stessi nomi di campo della versione C#).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_DELETE, VK_DOWN, VK_END, VK_F1, VK_HOME,
    VK_INSERT, VK_LEFT, VK_NEXT, VK_NUMPAD0, VK_PAUSE, VK_PRIOR, VK_RIGHT, VK_SPACE, VK_UP,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Settings {
    /// Cartella il cui contenuto diventa il menu.
    pub root_folder: PathBuf,
    /// Se false, file e cartelle nascosti o di sistema non vengono mostrati.
    pub show_hidden: bool,
    /// Scorciatoia globale per aprire il menu, es. "Ctrl+Alt+Space". Vuota = disattivata.
    pub hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        Self {
            root_folder: home.join("TrayMenu"),
            show_hidden: false,
            hotkey: "Ctrl+Alt+Space".into(),
        }
    }
}

impl Settings {
    pub fn file_path() -> PathBuf {
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
        appdata.join("FolderTrayMenu").join("settings.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::file_path())
            .ok()
            // File assente o corrotto: si riparte dai default.
            .and_then(|text| serde_json::from_str(text.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::file_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

// ----------------------------------------------------------- scorciatoie

/// Tasti con nome, oltre a lettere, cifre, F1–F24 e Num0–Num9.
const NAMED_KEYS: &[(&str, u16)] = &[
    ("Space", VK_SPACE),
    ("Insert", VK_INSERT),
    ("Delete", VK_DELETE),
    ("Home", VK_HOME),
    ("End", VK_END),
    ("PageUp", VK_PRIOR),
    ("PageDown", VK_NEXT),
    ("Up", VK_UP),
    ("Down", VK_DOWN),
    ("Left", VK_LEFT),
    ("Right", VK_RIGHT),
    ("Pause", VK_PAUSE),
];

/// Tasti che nel controllo hotkey di Windows richiedono il flag "extended".
pub fn is_extended_key(vk: u32) -> bool {
    [VK_INSERT, VK_DELETE, VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT]
        .iter()
        .any(|&k| k as u32 == vk)
}

fn key_from_name(name: &str) -> Option<u32> {
    if let Some(&(_, vk)) = NAMED_KEYS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
        return Some(vk as u32);
    }
    let upper = name.to_ascii_uppercase();
    if upper.len() == 1 && upper.as_bytes()[0].is_ascii_alphanumeric() {
        return Some(upper.as_bytes()[0] as u32);
    }
    if let Some(n) = upper.strip_prefix("NUM").and_then(|n| n.parse::<u32>().ok()).filter(|n| *n <= 9) {
        return Some(VK_NUMPAD0 as u32 + n);
    }
    upper
        .strip_prefix('F')
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| (1..=24).contains(n))
        .map(|n| VK_F1 as u32 + n - 1)
}

fn key_name(vk: u32) -> Option<String> {
    if let Some((name, _)) = NAMED_KEYS.iter().find(|(_, k)| *k as u32 == vk) {
        return Some((*name).to_string());
    }
    match vk {
        0x30..=0x39 | 0x41..=0x5A => Some(char::from(vk as u8).to_string()),
        v if (VK_NUMPAD0 as u32..=VK_NUMPAD0 as u32 + 9).contains(&v) => Some(format!("Num{}", v - VK_NUMPAD0 as u32)),
        v if (VK_F1 as u32..VK_F1 as u32 + 24).contains(&v) => Some(format!("F{}", v - VK_F1 as u32 + 1)),
        _ => None,
    }
}

/// Converte "Ctrl+Alt+Space" in (modificatori, virtual key). `None` se vuota o non valida.
pub fn parse_hotkey(text: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let mut mods: HOT_KEY_MODIFIERS = 0;
    let mut key: Option<u32> = None;

    for part in text.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "alt" => mods |= MOD_ALT,
            "shift" => mods |= MOD_SHIFT,
            "win" => mods |= MOD_WIN,
            other => key = Some(key_from_name(other)?),
        }
    }

    key.map(|k| (mods, k))
}

/// Inverso di `parse_hotkey`; `None` se il tasto non è tra quelli supportati.
pub fn format_hotkey(mods: HOT_KEY_MODIFIERS, vk: u32) -> Option<String> {
    let mut parts: Vec<String> = [(MOD_CONTROL, "Ctrl"), (MOD_ALT, "Alt"), (MOD_SHIFT, "Shift"), (MOD_WIN, "Win")]
        .iter()
        .filter(|(m, _)| mods & m != 0)
        .map(|(_, n)| (*n).to_string())
        .collect();
    parts.push(key_name(vk)?);
    Some(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_valide_e_non_valide() {
        assert_eq!(parse_hotkey("Ctrl+Alt+Space"), Some((MOD_CONTROL | MOD_ALT, VK_SPACE as u32)));
        assert_eq!(parse_hotkey("win + shift + m"), Some((MOD_WIN | MOD_SHIFT, 'M' as u32)));
        assert_eq!(parse_hotkey("Ctrl+F12"), Some((MOD_CONTROL, VK_F1 as u32 + 11)));
        assert_eq!(parse_hotkey("Ctrl+Num7"), Some((MOD_CONTROL, VK_NUMPAD0 as u32 + 7)));
        assert_eq!(parse_hotkey(""), None);
        assert_eq!(parse_hotkey("Ctrl+Alt"), None);
        assert_eq!(parse_hotkey("Ctrl+F25"), None);
        assert_eq!(parse_hotkey("Ctrl+Pippo"), None);
    }

    #[test]
    fn format_e_parse_sono_inversi() {
        for text in ["Ctrl+Alt+Space", "Shift+Win+M", "Ctrl+F12", "Alt+PageDown", "Ctrl+Shift+Num0", "Ctrl+7"] {
            let (mods, vk) = parse_hotkey(text).unwrap();
            let formatted = format_hotkey(mods, vk).unwrap();
            assert_eq!(parse_hotkey(&formatted), Some((mods, vk)), "{text} -> {formatted}");
        }
        assert_eq!(format_hotkey(MOD_CONTROL, 0xBA), None); // tasto non supportato
    }

    #[test]
    fn json_compatibile_con_versione_csharp() {
        let s: Settings = serde_json::from_str(r#"{ "RootFolder": "D:\\Link", "ShowHidden": true }"#).unwrap();
        assert_eq!(s.root_folder, PathBuf::from(r"D:\Link"));
        assert!(s.show_hidden);
        assert_eq!(s.hotkey, "Ctrl+Alt+Space");
    }
}
