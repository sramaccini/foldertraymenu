# FolderTrayMenu

Icona nella system tray di Windows che mostra il contenuto di una cartella come menu,
navigabile con mouse e tastiera. Le sottocartelle diventano sottomenu, letti solo quando
vengono aperti: il menu riflette sempre lo stato attuale del disco.

Alternativa leggera e open source a SystemTrayMenu: un unico eseguibile nativo (~250 KB),
senza runtime né installazione.

## Funzionalità

- Menu e sottomenu nativi di Windows, con le icone di Esplora file
- Ordinamento come Esplora file: prima le cartelle, poi i file, in ordine naturale (`file2` prima di `file10`)
- Estensione nascosta per collegamenti (`.lnk`, `.url`)
- Scorciatoia globale per aprire il menu (default `Ctrl+Alt+Space`)
- Ricerca per lettera iniziale all'interno del menu
- Finestra **Impostazioni** per scegliere cartella radice e scorciatoia
- Elementi nascosti mostrabili a richiesta, avvio automatico con Windows
- Una sola istanza; l'icona ricompare da sola se Esplora risorse viene riavviato
- Supporto per monitor ad alta risoluzione (DPI per monitor)

## Uso

| Azione | Mouse | Tastiera |
|---|---|---|
| Aprire il menu | clic sinistro o destro sull'icona | scorciatoia globale, oppure `Win+B`, frecce fino all'icona, `Invio` |
| Muoversi | — | `↑` `↓`; `→` entra in un sottomenu, `←` torna indietro |
| Cercare | — | una lettera porta alla voce successiva che inizia con quella lettera; ripetendola si scorre |
| Aprire file o collegamento | clic | `Invio` |
| Aprire una cartella in Esplora file | voce "Apri in Esplora file" in cima al sottomenu | idem |
| Chiudere | clic fuori dal menu | `Esc` |

In fondo al menu, il sottomenu **FolderTrayMenu** contiene: apri cartella radice,
Impostazioni…, mostra elementi nascosti, avvia con Windows, esci.

### Impostazioni

Dalla voce **Impostazioni…** si sceglie:

- **Cartella radice**: si digita il percorso o si usa **Sfoglia…**
- **Scorciatoia**: si fa clic nel campo e si preme la combinazione desiderata (almeno uno tra Ctrl, Alt e Shift).
  **Nessuna** disattiva la scorciatoia. Se la combinazione è già usata da un'altra applicazione viene segnalato
  e resta attiva quella precedente.

`Tab` passa da un campo all'altro, `Invio` conferma, `Esc` annulla.

### File di configurazione

Le impostazioni sono salvate in `%APPDATA%\FolderTrayMenu\settings.json`:

```json
{
  "RootFolder": "C:\\Users\\<utente>\\TrayMenu",
  "ShowHidden": false,
  "Hotkey": "Ctrl+Alt+Space"
}
```

`Hotkey` accetta i modificatori `Ctrl`, `Alt`, `Shift`, `Win` e come tasto una lettera, una cifra,
`F1`–`F24`, `Num0`–`Num9`, `Space`, `Insert`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`,
`Up`, `Down`, `Left`, `Right` o `Pause`. Il tasto `Win` si può impostare solo modificando il file.
Le modifiche manuali al file valgono dal riavvio successivo.

La cartella radice si può passare anche da riga di comando: `FolderTrayMenu.exe "D:\Link"`.

## Compilazione

Requisiti: toolchain Rust MSVC (`rustup`) e Visual Studio Build Tools con i componenti C++ e il Windows SDK.

```
cargo build --release
cargo test
```

L'eseguibile si trova in `target\release\FolderTrayMenu.exe`.

## Licenza

Distribuito con licenza [Mozilla Public License 2.0](LICENSE).
