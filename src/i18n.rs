//! Interface translations.
//!
//! The language follows the Windows display language (English when it is not
//! supported) and can be forced through the `Language` setting.
//! Texts with parameters use `{0}`, `{1}` placeholders, filled in by [`fmt`].
//! A `&` marks the keyboard mnemonic (underlined letter).

use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};

use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    It,
    De,
    Fr,
    Es,
    Pt,
    Nl,
    Pl,
}

pub const ALL: [Lang; 8] = [Lang::En, Lang::It, Lang::De, Lang::Fr, Lang::Es, Lang::Pt, Lang::Nl, Lang::Pl];

impl Lang {
    /// Code stored in `settings.json`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::It => "it",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::Es => "es",
            Lang::Pt => "pt",
            Lang::Nl => "nl",
            Lang::Pl => "pl",
        }
    }

    /// Name of the language in the language itself, as shown in the selector.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::It => "Italiano",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Es => "Español",
            Lang::Pt => "Português",
            Lang::Nl => "Nederlands",
            Lang::Pl => "Polski",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let code = code.trim().to_ascii_lowercase();
        let primary = code.split(['-', '_']).next().unwrap_or_default();
        ALL.into_iter().find(|l| l.code() == primary)
    }

    fn strings(self) -> &'static Strings {
        match self {
            Lang::En => &EN,
            Lang::It => &IT,
            Lang::De => &DE,
            Lang::Fr => &FR,
            Lang::Es => &ES,
            Lang::Pt => &PT,
            Lang::Nl => &NL,
            Lang::Pl => &PL,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Sets the active language from the setting value; empty or unknown = Windows language.
pub fn set_language(setting: &str) {
    let lang = Lang::from_code(setting).unwrap_or_else(system_language);
    let index = ALL.iter().position(|&l| l == lang).unwrap_or(0);
    CURRENT.store(index as u8, Ordering::Relaxed);
}

/// Windows display language, if supported; English otherwise.
pub fn system_language() -> Lang {
    // Primary language identifier (low 10 bits of the LANGID).
    match unsafe { GetUserDefaultUILanguage() } & 0x3FF {
        0x10 => Lang::It,
        0x07 => Lang::De,
        0x0C => Lang::Fr,
        0x0A => Lang::Es,
        0x16 => Lang::Pt,
        0x13 => Lang::Nl,
        0x15 => Lang::Pl,
        _ => Lang::En,
    }
}

/// Texts of the active language.
pub fn t() -> &'static Strings {
    ALL[CURRENT.load(Ordering::Relaxed) as usize % ALL.len()].strings()
}

/// Replaces `{0}`, `{1}`, … with the given arguments.
pub fn fmt(template: &str, args: &[&dyn Display]) -> String {
    args.iter()
        .enumerate()
        .fold(template.to_string(), |text, (i, arg)| text.replace(&format!("{{{i}}}"), &arg.to_string()))
}

pub struct Strings {
    // Tray menu
    pub folder_not_found: &'static str,
    pub menu_open_root: &'static str,
    pub menu_settings: &'static str,
    pub menu_show_hidden: &'static str,
    pub menu_autostart: &'static str,
    pub menu_hotkey: &'static str,
    pub menu_hotkey_none: &'static str,
    pub menu_exit: &'static str,
    pub open_in_explorer: &'static str,
    pub cannot_read: &'static str,
    pub empty_folder: &'static str,
    // Messages
    pub folder_missing: &'static str,
    pub hotkey_invalid: &'static str,
    pub hotkey_in_use: &'static str,
    pub apply_busy: &'static str,
    pub open_failed: &'static str,
    pub pick_folder_title: &'static str,
    // Settings window
    pub settings_title: &'static str,
    pub root_folder_label: &'static str,
    pub browse: &'static str,
    pub hotkey_label: &'static str,
    pub hotkey_clear: &'static str,
    pub hotkey_hint: &'static str,
    pub hotkey_hint_win: &'static str,
    pub language_label: &'static str,
    pub language_auto: &'static str,
    pub ok: &'static str,
    pub cancel: &'static str,
    pub folder_required: &'static str,
    pub key_unsupported: &'static str,
}

static EN: Strings = Strings {
    folder_not_found: "Folder not found: {0}",
    menu_open_root: "&Open root folder",
    menu_settings: "&Settings…",
    menu_show_hidden: "Show &hidden items",
    menu_autostart: "Start with &Windows",
    menu_hotkey: "Hotkey: {0}",
    menu_hotkey_none: "Hotkey: not set",
    menu_exit: "E&xit",
    open_in_explorer: "Open in File Explorer",
    cannot_read: "(cannot read: {0})",
    empty_folder: "(empty)",
    folder_missing: "The folder does not exist:\n{0}",
    hotkey_invalid: "Invalid hotkey: {0}",
    hotkey_in_use: "The hotkey {0} is already used by another application.\nPlease choose a different one.",
    apply_busy: "The settings cannot be applied right now, please try again.",
    open_failed: "Cannot open:\n{0}\n\nError code: {1}",
    pick_folder_title: "Folder to show in the menu",
    settings_title: "Settings – FolderTrayMenu",
    root_folder_label: "&Root folder",
    browse: "&Browse…",
    hotkey_label: "&Hotkey to open the menu",
    hotkey_clear: "&None",
    hotkey_hint: "Click in the field and press the combination (with Ctrl, Alt or Shift). “None” disables the hotkey.",
    hotkey_hint_win: "The current hotkey is {0}: this field cannot show the Win key, \
                      but the hotkey is kept if you leave it unchanged.",
    language_label: "&Language",
    language_auto: "Automatic (Windows language)",
    ok: "OK",
    cancel: "Cancel",
    folder_required: "Please enter the root folder.",
    key_unsupported: "This key is not supported for the hotkey.\n\
                      Use a letter, a digit, F1–F24, the numeric keypad, Space, the arrow keys, \
                      Insert, Delete, Home, End, Page Up, Page Down or Pause.",
};

static IT: Strings = Strings {
    folder_not_found: "Cartella non trovata: {0}",
    menu_open_root: "&Apri cartella radice",
    menu_settings: "&Impostazioni…",
    menu_show_hidden: "Mostra elementi &nascosti",
    menu_autostart: "Avvia con &Windows",
    menu_hotkey: "Scorciatoia: {0}",
    menu_hotkey_none: "Scorciatoia: non attiva",
    menu_exit: "&Esci",
    open_in_explorer: "Apri in Esplora file",
    cannot_read: "(impossibile leggere: {0})",
    empty_folder: "(vuota)",
    folder_missing: "La cartella non esiste:\n{0}",
    hotkey_invalid: "Scorciatoia non valida: {0}",
    hotkey_in_use: "La scorciatoia {0} è già usata da un'altra applicazione.\nScegline un'altra.",
    apply_busy: "Impossibile applicare le impostazioni in questo momento, riprova.",
    open_failed: "Impossibile aprire:\n{0}\n\nCodice errore: {1}",
    pick_folder_title: "Cartella da mostrare nel menu",
    settings_title: "Impostazioni – FolderTrayMenu",
    root_folder_label: "&Cartella radice",
    browse: "&Sfoglia…",
    hotkey_label: "Scorciatoia per aprire il &menu",
    hotkey_clear: "&Nessuna",
    hotkey_hint: "Fai clic nel campo e premi la combinazione (con Ctrl, Alt o Shift). «Nessuna» disattiva la scorciatoia.",
    hotkey_hint_win: "La scorciatoia attuale è {0}: il campo non può mostrare il tasto Win, \
                      ma viene mantenuta se non la modifichi.",
    language_label: "&Lingua",
    language_auto: "Automatica (lingua di Windows)",
    ok: "OK",
    cancel: "Annulla",
    folder_required: "Indica la cartella radice.",
    key_unsupported: "Tasto non supportato per la scorciatoia.\n\
                      Usa una lettera, una cifra, F1–F24, il tastierino numerico, Spazio, le frecce, \
                      Ins, Canc, Home, Fine, PagSu, PagGiù o Pausa.",
};

static DE: Strings = Strings {
    folder_not_found: "Ordner nicht gefunden: {0}",
    menu_open_root: "Stammordner ö&ffnen",
    menu_settings: "&Einstellungen…",
    menu_show_hidden: "&Versteckte Elemente anzeigen",
    menu_autostart: "Mit &Windows starten",
    menu_hotkey: "Tastenkombination: {0}",
    menu_hotkey_none: "Tastenkombination: nicht festgelegt",
    menu_exit: "&Beenden",
    open_in_explorer: "Im Datei-Explorer öffnen",
    cannot_read: "(nicht lesbar: {0})",
    empty_folder: "(leer)",
    folder_missing: "Der Ordner existiert nicht:\n{0}",
    hotkey_invalid: "Ungültige Tastenkombination: {0}",
    hotkey_in_use: "Die Tastenkombination {0} wird bereits von einer anderen Anwendung verwendet.\n\
                    Bitte wählen Sie eine andere.",
    apply_busy: "Die Einstellungen können gerade nicht übernommen werden. Bitte versuchen Sie es erneut.",
    open_failed: "Öffnen nicht möglich:\n{0}\n\nFehlercode: {1}",
    pick_folder_title: "Im Menü anzuzeigender Ordner",
    settings_title: "Einstellungen – FolderTrayMenu",
    root_folder_label: "&Stammordner",
    browse: "&Durchsuchen…",
    hotkey_label: "&Tastenkombination zum Öffnen des Menüs",
    hotkey_clear: "&Keine",
    hotkey_hint: "Klicken Sie in das Feld und drücken Sie die Kombination (mit Strg, Alt oder Umschalt). \
                  „Keine“ deaktiviert die Tastenkombination.",
    hotkey_hint_win: "Die aktuelle Tastenkombination ist {0}: Das Feld kann die Windows-Taste nicht anzeigen, \
                      die Kombination bleibt aber erhalten, wenn Sie sie nicht ändern.",
    language_label: "S&prache",
    language_auto: "Automatisch (Windows-Sprache)",
    ok: "OK",
    cancel: "Abbrechen",
    folder_required: "Bitte geben Sie den Stammordner an.",
    key_unsupported: "Diese Taste wird für die Tastenkombination nicht unterstützt.\n\
                      Verwenden Sie einen Buchstaben, eine Ziffer, F1–F24, den Ziffernblock, die Leertaste, \
                      die Pfeiltasten, Einfg, Entf, Pos1, Ende, Bild auf, Bild ab oder Pause.",
};

static FR: Strings = Strings {
    folder_not_found: "Dossier introuvable\u{a0}: {0}",
    menu_open_root: "&Ouvrir le dossier racine",
    menu_settings: "&Paramètres…",
    menu_show_hidden: "Afficher les éléments &masqués",
    menu_autostart: "Lancer au démarrage de &Windows",
    menu_hotkey: "Raccourci\u{a0}: {0}",
    menu_hotkey_none: "Raccourci\u{a0}: aucun",
    menu_exit: "&Quitter",
    open_in_explorer: "Ouvrir dans l'Explorateur de fichiers",
    cannot_read: "(lecture impossible\u{a0}: {0})",
    empty_folder: "(vide)",
    folder_missing: "Le dossier n'existe pas\u{a0}:\n{0}",
    hotkey_invalid: "Raccourci non valide\u{a0}: {0}",
    hotkey_in_use: "Le raccourci {0} est déjà utilisé par une autre application.\nVeuillez en choisir un autre.",
    apply_busy: "Impossible d'appliquer les paramètres pour le moment, veuillez réessayer.",
    open_failed: "Impossible d'ouvrir\u{a0}:\n{0}\n\nCode d'erreur\u{a0}: {1}",
    pick_folder_title: "Dossier à afficher dans le menu",
    settings_title: "Paramètres – FolderTrayMenu",
    root_folder_label: "&Dossier racine",
    browse: "&Parcourir…",
    hotkey_label: "&Raccourci pour ouvrir le menu",
    hotkey_clear: "&Aucun",
    hotkey_hint: "Cliquez dans le champ et appuyez sur la combinaison (avec Ctrl, Alt ou Maj). \
                  «\u{a0}Aucun\u{a0}» désactive le raccourci.",
    hotkey_hint_win: "Le raccourci actuel est {0}\u{a0}: le champ ne peut pas afficher la touche Windows, \
                      mais le raccourci est conservé si vous ne le modifiez pas.",
    language_label: "&Langue",
    language_auto: "Automatique (langue de Windows)",
    ok: "OK",
    cancel: "Annuler",
    folder_required: "Veuillez indiquer le dossier racine.",
    key_unsupported: "Cette touche n'est pas prise en charge pour le raccourci.\n\
                      Utilisez une lettre, un chiffre, F1–F24, le pavé numérique, Espace, les flèches, \
                      Inser, Suppr, Origine, Fin, Page préc., Page suiv. ou Pause.",
};

static ES: Strings = Strings {
    folder_not_found: "No se encuentra la carpeta: {0}",
    menu_open_root: "&Abrir carpeta raíz",
    menu_settings: "&Configuración…",
    menu_show_hidden: "Mostrar elementos &ocultos",
    menu_autostart: "Iniciar con &Windows",
    menu_hotkey: "Atajo: {0}",
    menu_hotkey_none: "Atajo: no definido",
    menu_exit: "&Salir",
    open_in_explorer: "Abrir en el Explorador de archivos",
    cannot_read: "(no se puede leer: {0})",
    empty_folder: "(vacía)",
    folder_missing: "La carpeta no existe:\n{0}",
    hotkey_invalid: "Atajo no válido: {0}",
    hotkey_in_use: "El atajo {0} ya lo usa otra aplicación.\nElige otro.",
    apply_busy: "No se puede aplicar la configuración en este momento; inténtalo de nuevo.",
    open_failed: "No se puede abrir:\n{0}\n\nCódigo de error: {1}",
    pick_folder_title: "Carpeta que se mostrará en el menú",
    settings_title: "Configuración – FolderTrayMenu",
    root_folder_label: "&Carpeta raíz",
    browse: "&Examinar…",
    hotkey_label: "&Atajo para abrir el menú",
    hotkey_clear: "&Ninguno",
    hotkey_hint: "Haz clic en el campo y pulsa la combinación (con Ctrl, Alt o Mayús). «Ninguno» desactiva el atajo.",
    hotkey_hint_win: "El atajo actual es {0}: el campo no puede mostrar la tecla Windows, \
                      pero el atajo se conserva si no lo cambias.",
    language_label: "&Idioma",
    language_auto: "Automático (idioma de Windows)",
    ok: "Aceptar",
    cancel: "Cancelar",
    folder_required: "Indica la carpeta raíz.",
    key_unsupported: "Esta tecla no se admite para el atajo.\n\
                      Usa una letra, un número, F1–F24, el teclado numérico, Espacio, las flechas, \
                      Insert, Supr, Inicio, Fin, RePág, AvPág o Pausa.",
};

static PT: Strings = Strings {
    folder_not_found: "Pasta não encontrada: {0}",
    menu_open_root: "&Abrir pasta raiz",
    menu_settings: "&Definições…",
    menu_show_hidden: "Mostrar itens &ocultos",
    menu_autostart: "Iniciar com o &Windows",
    menu_hotkey: "Atalho: {0}",
    menu_hotkey_none: "Atalho: não definido",
    menu_exit: "&Sair",
    open_in_explorer: "Abrir no Explorador de Ficheiros",
    cannot_read: "(não é possível ler: {0})",
    empty_folder: "(vazia)",
    folder_missing: "A pasta não existe:\n{0}",
    hotkey_invalid: "Atalho inválido: {0}",
    hotkey_in_use: "O atalho {0} já está a ser utilizado por outra aplicação.\nEscolha outro.",
    apply_busy: "Não é possível aplicar as definições neste momento. Tente novamente.",
    open_failed: "Não é possível abrir:\n{0}\n\nCódigo de erro: {1}",
    pick_folder_title: "Pasta a mostrar no menu",
    settings_title: "Definições – FolderTrayMenu",
    root_folder_label: "&Pasta raiz",
    browse: "Pro&curar…",
    hotkey_label: "A&talho para abrir o menu",
    hotkey_clear: "&Nenhum",
    hotkey_hint: "Clique no campo e prima a combinação (com Ctrl, Alt ou Shift). «Nenhum» desativa o atalho.",
    hotkey_hint_win: "O atalho atual é {0}: o campo não consegue mostrar a tecla Windows, \
                      mas o atalho é mantido se não o alterar.",
    language_label: "&Idioma",
    language_auto: "Automático (idioma do Windows)",
    ok: "OK",
    cancel: "Cancelar",
    folder_required: "Indique a pasta raiz.",
    key_unsupported: "Esta tecla não é suportada para o atalho.\n\
                      Utilize uma letra, um algarismo, F1–F24, o teclado numérico, Espaço, as setas, \
                      Insert, Delete, Home, End, Page Up, Page Down ou Pause.",
};

static NL: Strings = Strings {
    folder_not_found: "Map niet gevonden: {0}",
    menu_open_root: "&Hoofdmap openen",
    menu_settings: "&Instellingen…",
    menu_show_hidden: "&Verborgen items weergeven",
    menu_autostart: "Starten met &Windows",
    menu_hotkey: "Sneltoets: {0}",
    menu_hotkey_none: "Sneltoets: niet ingesteld",
    menu_exit: "&Afsluiten",
    open_in_explorer: "Openen in Verkenner",
    cannot_read: "(kan niet worden gelezen: {0})",
    empty_folder: "(leeg)",
    folder_missing: "De map bestaat niet:\n{0}",
    hotkey_invalid: "Ongeldige sneltoets: {0}",
    hotkey_in_use: "De sneltoets {0} wordt al door een andere toepassing gebruikt.\nKies een andere.",
    apply_busy: "De instellingen kunnen nu niet worden toegepast. Probeer het opnieuw.",
    open_failed: "Kan niet openen:\n{0}\n\nFoutcode: {1}",
    pick_folder_title: "Map die in het menu wordt weergegeven",
    settings_title: "Instellingen – FolderTrayMenu",
    root_folder_label: "&Hoofdmap",
    browse: "&Bladeren…",
    hotkey_label: "&Sneltoets om het menu te openen",
    hotkey_clear: "&Geen",
    hotkey_hint: "Klik in het veld en druk op de combinatie (met Ctrl, Alt of Shift). ‘Geen’ schakelt de sneltoets uit.",
    hotkey_hint_win: "De huidige sneltoets is {0}: het veld kan de Windows-toets niet weergeven, \
                      maar de sneltoets blijft behouden als u hem niet wijzigt.",
    language_label: "&Taal",
    language_auto: "Automatisch (taal van Windows)",
    ok: "OK",
    cancel: "Annuleren",
    folder_required: "Geef de hoofdmap op.",
    key_unsupported: "Deze toets wordt niet ondersteund voor de sneltoets.\n\
                      Gebruik een letter, een cijfer, F1–F24, het numerieke toetsenblok, de spatiebalk, \
                      de pijltoetsen, Insert, Delete, Home, End, Page Up, Page Down of Pause.",
};

static PL: Strings = Strings {
    folder_not_found: "Nie znaleziono folderu: {0}",
    menu_open_root: "&Otwórz folder główny",
    menu_settings: "&Ustawienia…",
    menu_show_hidden: "Pokaż ukryte &elementy",
    menu_autostart: "Uruchamiaj z systemem &Windows",
    menu_hotkey: "Skrót: {0}",
    menu_hotkey_none: "Skrót: nieustawiony",
    menu_exit: "&Zakończ",
    open_in_explorer: "Otwórz w Eksploratorze plików",
    cannot_read: "(nie można odczytać: {0})",
    empty_folder: "(pusty)",
    folder_missing: "Folder nie istnieje:\n{0}",
    hotkey_invalid: "Nieprawidłowy skrót: {0}",
    hotkey_in_use: "Skrót {0} jest już używany przez inną aplikację.\nWybierz inny.",
    apply_busy: "Nie można teraz zastosować ustawień. Spróbuj ponownie.",
    open_failed: "Nie można otworzyć:\n{0}\n\nKod błędu: {1}",
    pick_folder_title: "Folder wyświetlany w menu",
    settings_title: "Ustawienia – FolderTrayMenu",
    root_folder_label: "&Folder główny",
    browse: "&Przeglądaj…",
    hotkey_label: "&Skrót otwierający menu",
    hotkey_clear: "&Brak",
    hotkey_hint: "Kliknij pole i naciśnij kombinację (z Ctrl, Alt lub Shift). „Brak” wyłącza skrót.",
    hotkey_hint_win: "Bieżący skrót to {0}: pole nie może wyświetlić klawisza Windows, \
                      ale skrót zostanie zachowany, jeśli go nie zmienisz.",
    language_label: "&Język",
    language_auto: "Automatycznie (język systemu Windows)",
    ok: "OK",
    cancel: "Anuluj",
    folder_required: "Podaj folder główny.",
    key_unsupported: "Ten klawisz nie jest obsługiwany w skrócie.\n\
                      Użyj litery, cyfry, F1–F24, klawiatury numerycznej, spacji, strzałek, \
                      Insert, Delete, Home, End, Page Up, Page Down lub Pause.",
};

#[cfg(test)]
mod tests {
    use super::*;

    fn mnemonic(text: &str) -> Option<char> {
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '&' {
                return chars.next().map(|m| m.to_lowercase().next().unwrap());
            }
        }
        None
    }

    fn assert_unique(lang: Lang, group: &str, texts: &[&str]) {
        let mut seen = Vec::new();
        for text in texts {
            let m = mnemonic(text).unwrap_or_else(|| panic!("{lang:?} {group}: missing mnemonic in {text:?}"));
            assert!(!seen.contains(&m), "{lang:?} {group}: duplicate mnemonic '{m}' in {text:?}");
            seen.push(m);
        }
    }

    #[test]
    fn mnemonics_are_unique_within_each_menu_and_window() {
        for lang in ALL {
            let s = lang.strings();
            assert_unique(
                lang,
                "menu",
                &[s.menu_open_root, s.menu_settings, s.menu_show_hidden, s.menu_autostart, s.menu_exit],
            );
            assert_unique(
                lang,
                "settings",
                &[s.root_folder_label, s.browse, s.hotkey_label, s.hotkey_clear, s.language_label],
            );
        }
    }

    #[test]
    fn placeholders_are_present_in_every_language() {
        for lang in ALL {
            let s = lang.strings();
            for (name, text, count) in [
                ("folder_not_found", s.folder_not_found, 1),
                ("menu_hotkey", s.menu_hotkey, 1),
                ("cannot_read", s.cannot_read, 1),
                ("folder_missing", s.folder_missing, 1),
                ("hotkey_invalid", s.hotkey_invalid, 1),
                ("hotkey_in_use", s.hotkey_in_use, 1),
                ("open_failed", s.open_failed, 2),
                ("hotkey_hint_win", s.hotkey_hint_win, 1),
            ] {
                for i in 0..count {
                    assert!(text.contains(&format!("{{{i}}}")), "{lang:?}.{name}: missing {{{i}}}");
                }
            }
        }
    }

    #[test]
    fn codes_and_formatting() {
        assert_eq!(Lang::from_code("pt-BR"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("DE"), Some(Lang::De));
        assert_eq!(Lang::from_code(""), None);
        assert_eq!(Lang::from_code("sv"), None);
        assert_eq!(fmt("a {0} b {1} {0}", &[&1, &"x"]), "a 1 b x 1");
    }
}
