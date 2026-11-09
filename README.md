# FolderTrayMenu

A Windows system tray icon that shows the contents of a folder as a menu,
navigable with both mouse and keyboard. Subfolders become submenus, read only when
they are opened, so the menu always reflects the current state of the disk.

A lightweight, open source alternative to SystemTrayMenu: a single native executable (~250 KB),
no runtime and no installation required.

## Download

**[⬇ FolderTrayMenu.exe](https://github.com/sramaccini/foldertraymenu/releases/latest/download/FolderTrayMenu.exe)**
(latest release, Windows 10/11 x64) — see all [releases](https://github.com/sramaccini/foldertraymenu/releases).

Save the file anywhere and run it; to start it automatically, use *Start with Windows* in its menu.
The executable is not code-signed, so Windows SmartScreen may show a warning the first time:
choose *More info* → *Run anyway*. The SHA-256 checksum of each file is listed in the release notes.

## Features

- Native Windows menus and submenus, with the same icons as File Explorer
- File Explorer ordering: folders first, then files, in natural order (`file2` before `file10`)
- Hidden extension for shortcuts (`.lnk`, `.url`)
- Global hotkey to open the menu (default `Ctrl+Alt+Space`)
- Search by first letter inside the menu
- **Settings** window to choose the root folder, the hotkey and the language
- User interface in English, Italian, German, French, Spanish, Portuguese, Dutch and Polish
- Optionally show hidden items; start with Windows
- Single instance; the icon comes back automatically if Windows Explorer restarts
- High-DPI support (per-monitor DPI awareness)

## Usage

| Action | Mouse | Keyboard |
|---|---|---|
| Open the menu | left or right click on the icon | global hotkey, or `Win+B`, arrow keys to the icon, `Enter` |
| Move around | — | `↑` `↓`; `→` enters a submenu, `←` goes back |
| Search | — | a letter jumps to the next item starting with that letter; press it again to cycle |
| Open a file or shortcut | click | `Enter` |
| Open a folder in File Explorer | "Open in File Explorer" item at the top of the submenu | same |
| Close | click outside the menu | `Esc` |

At the bottom of the menu, the **FolderTrayMenu** submenu contains: open root folder,
Settings…, show hidden items, start with Windows, exit.

### Settings

The **Settings…** item lets you choose:

- **Root folder**: type the path or use **Browse…**
- **Hotkey**: click in the field and press the desired combination (at least one of Ctrl, Alt and Shift).
  **None** disables the hotkey. If the combination is already used by another application,
  you are told so and the previous hotkey stays active.
- **Language**: *Automatic* follows the Windows display language (English if it is not supported),
  or pick one of the available languages. The change applies immediately.

`Tab` moves between fields, `Enter` confirms, `Esc` cancels.

### Languages

| Code | Language |
|---|---|
| `en` | English |
| `it` | Italiano |
| `de` | Deutsch |
| `fr` | Français |
| `es` | Español |
| `pt` | Português |
| `nl` | Nederlands |
| `pl` | Polski |

Translations live in [`src/i18n.rs`](src/i18n.rs). To add a language, add a `Lang` variant,
a `Strings` table and the Windows primary language ID in `system_language`; the tests check
that every language has all placeholders and unique keyboard mnemonics.
Corrections from native speakers are welcome.

### Configuration file

Settings are saved in `%APPDATA%\FolderTrayMenu\settings.json`:

```json
{
  "RootFolder": "C:\\Users\\<user>\\TrayMenu",
  "ShowHidden": false,
  "Hotkey": "Ctrl+Alt+Space",
  "Language": ""
}
```

`Hotkey` accepts the modifiers `Ctrl`, `Alt`, `Shift`, `Win` and, as the key, a letter, a digit,
`F1`–`F24`, `Num0`–`Num9`, `Space`, `Insert`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`,
`Up`, `Down`, `Left`, `Right` or `Pause`. The `Win` key can only be set by editing the file.

`Language` is one of the codes above; empty means the Windows display language.

Manual changes to the file take effect at the next start.

The root folder can also be passed on the command line: `FolderTrayMenu.exe "D:\Links"`.

## Building

Requirements: the Rust MSVC toolchain (`rustup`) and Visual Studio Build Tools with the C++ components and the Windows SDK.

```
cargo build --release
cargo test
```

The executable is written to `target\release\FolderTrayMenu.exe`.

## License

Distributed under the [Mozilla Public License 2.0](LICENSE).
