# FolderTrayMenu

A Windows system tray icon that shows the contents of a folder as a menu,
navigable with both mouse and keyboard. Subfolders become submenus, read only when
they are opened, so the menu always reflects the current state of the disk.

A lightweight, open source alternative to SystemTrayMenu: a single native executable (~250 KB),
no runtime and no installation required.

## Features

- Native Windows menus and submenus, with the same icons as File Explorer
- File Explorer ordering: folders first, then files, in natural order (`file2` before `file10`)
- Hidden extension for shortcuts (`.lnk`, `.url`)
- Global hotkey to open the menu (default `Ctrl+Alt+Space`)
- Search by first letter inside the menu
- **Settings** window to choose the root folder and the hotkey
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
| Open a folder in File Explorer | "Apri in Esplora file" item at the top of the submenu | same |
| Close | click outside the menu | `Esc` |

At the bottom of the menu, the **FolderTrayMenu** submenu contains: open root folder,
Settings…, show hidden items, start with Windows, exit.

> The user interface is currently in Italian.

### Settings

The **Impostazioni…** (Settings) item lets you choose:

- **Root folder**: type the path or use **Sfoglia…** (Browse)
- **Hotkey**: click in the field and press the desired combination (at least one of Ctrl, Alt and Shift).
  **Nessuna** (None) disables the hotkey. If the combination is already used by another application,
  you are told so and the previous hotkey stays active.

`Tab` moves between fields, `Enter` confirms, `Esc` cancels.

### Configuration file

Settings are saved in `%APPDATA%\FolderTrayMenu\settings.json`:

```json
{
  "RootFolder": "C:\\Users\\<user>\\TrayMenu",
  "ShowHidden": false,
  "Hotkey": "Ctrl+Alt+Space"
}
```

`Hotkey` accepts the modifiers `Ctrl`, `Alt`, `Shift`, `Win` and, as the key, a letter, a digit,
`F1`–`F24`, `Num0`–`Num9`, `Space`, `Insert`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`,
`Up`, `Down`, `Left`, `Right` or `Pause`. The `Win` key can only be set by editing the file.
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
