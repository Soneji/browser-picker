# Browser Picker

A tiny, **native** browser picker for Windows. Set it as your default browser and
every clicked link pops up a small window asking *which* browser to open it in.

Inspired by [Browserino](https://github.com/AlexStrNik/Browserino) (macOS) and
[Browserosaurus](https://github.com/will-stone/browserosaurus) /
[browseratops](https://github.com/riotrah/browseratops), but built for Windows in
native Rust instead of Electron.

## Why another one?

- **Actually detects your browsers.** Reads `Clients\StartMenuInternet` from
  **all three** registry locations — `HKCU`, `HKLM`, and `HKLM\WOW6432Node` — and
  de-duplicates by executable path. No hard-coded list of "known" browsers. This
  is the thing Electron/mac-heritage ports get wrong: per-user installs (Chrome,
  Edge Beta/Dev/Canary, Brave, …) live in **HKCU**, which single-hive detection
  misses.
- **Lightweight.** A single native `.exe`, no bundled Chromium, no webview.
- **Zero background process.** Nothing runs in the tray or in the background. The
  exe only runs for the couple of seconds you're choosing a browser, then exits.
  Idle footprint is literally zero.
- **No admin required.** Registers itself under `HKCU`.

## Install

1. Download `browser-picker.exe` from the
   [Releases page](https://github.com/Soneji/browser-picker/releases) (or the
   latest [Actions build](https://github.com/Soneji/browser-picker/actions)).
2. Put it somewhere permanent, e.g. `%LOCALAPPDATA%\BrowserPicker\browser-picker.exe`.
   (Windows records the path you register from, so pick its home first.)
3. Double-click it, then click **Set as default browser**. That registers the app
   and opens **Settings ▸ Apps ▸ Default apps** — find **Browser Picker** and set
   it for `HTTP` and `HTTPS`.

That's it. Now any link opened from a non-browser app shows the picker.

## Usage

```
browser-picker.exe <url>          Show the picker for a URL (what Windows calls)
browser-picker.exe                Open the home / settings window
browser-picker.exe --register     Register as a selectable browser (no admin)
browser-picker.exe --unregister   Remove the registration
browser-picker.exe --list         List the browsers that were detected
browser-picker.exe --help         Show help
```

In the picker: click a browser, press **Alt+1 … Alt+9**, or press **Esc** / the
window's ✕ to cancel.

## Build from source

Real (release) binary — on Windows, or in CI on a `windows-latest` runner:

```
cargo build --release            # -> target\release\browser-picker.exe
```

Type-check on Linux/macOS without a Windows toolchain (no linking, so no mingw
needed):

```
rustup target add x86_64-pc-windows-gnu
cargo check --target x86_64-pc-windows-gnu
```

The GitHub Actions workflow (`.github/workflows/build.yml`) builds the MSVC
binary, prints its size, uploads it as an artifact on every push, and attaches it
to a GitHub Release when you push a `v*` tag.

## How detection works

For each of `HKCU\SOFTWARE\Clients\StartMenuInternet`,
`HKLM\SOFTWARE\Clients\StartMenuInternet`, and
`HKLM\SOFTWARE\WOW6432Node\Clients\StartMenuInternet`:

- enumerate the sub-keys (each is a registered internet client / browser),
- read the display name from `Capabilities\ApplicationName` (falling back to the
  key's default value),
- read the launch command from `shell\open\command`,
- parse the executable out of that command.

Then de-duplicate by exe path (a browser can appear in more than one hive) and
drop Browser Picker itself. Browsers are launched as `exe <url>`, which every
mainstream browser accepts.

## Roadmap

- Browser icons in the picker (via `ExtractIconEx` on each exe).
- Remember last choice / per-domain rules ("always open github.com in Firefox").
- Optional system-tray mode and global hotkey.
- Copy-URL-to-clipboard button.

## Credits

- [Browserino](https://github.com/AlexStrNik/Browserino) — the native-macOS
  inspiration.
- [Browserosaurus](https://github.com/will-stone/browserosaurus) &
  [browseratops](https://github.com/riotrah/browseratops) — the original idea and
  the Windows-port attempt.

## License

MIT.
