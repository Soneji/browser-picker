# Browser Picker

A tiny, native browser picker for Windows with a **modern flat UI**. Set it as
your default browser and every clicked link pops up a small window asking *which*
browser to open it in.

Inspired by [Browserino](https://github.com/AlexStrNik/Browserino) (macOS) and
[Browserosaurus](https://github.com/will-stone/browserosaurus) /
[browseratops](https://github.com/riotrah/browseratops) — but built for Windows
in native Rust, not Electron.

## Why another one?

- **Actually detects your browsers.** Reads `Clients\StartMenuInternet` from
  **all three** registry locations — `HKCU`, `HKLM`, and `HKLM\WOW6432Node` — and
  de-duplicates by executable path, with no hard-coded browser list. Per-user
  installs (Chrome, Edge Beta/Dev/Canary, Brave, …) live in **HKCU**, which
  single-hive detection misses.
- **Modern UI, no GPU.** A flat, dark, rounded window drawn directly with GDI —
  not a Windows-2000 dialog, and it needs no graphics adapter, so it runs on RDP
  sessions and VMs where GPU toolkits fail.
- **Icons, custom order, and a favourite.** The picker shows each browser's icon.
  Open the app to reorder browsers and star a favourite/default: the picker then
  lists them in your order, badges the favourite, and opens it on Enter.
- **Brings the browser to the front.** Grants foreground rights to the browser it
  launches, so the link opens on top instead of behind other windows.
- **No background process.** Nothing sits in the tray. The exe only runs for the
  couple of seconds you're choosing, then exits.
- **No admin required.** Registers under `HKCU`.

## Install

### Installer (recommended)

Download **`browser-picker-setup.exe`** from the
[Releases page](https://github.com/Soneji/browser-picker/releases) and run it.
It's a per-user install (no admin/UAC), adds a Start Menu entry and an uninstaller
(Add/Remove Programs), registers itself automatically, and offers to open Default
Apps so you can set it as default.

### Portable

Download **`browser-picker.exe`**, put it somewhere permanent, run it, and click
**Set as default browser**.

Either way, finish in **Settings ▸ Apps ▸ Default apps** by setting Browser Picker
for `HTTP` and `HTTPS`.

## Usage

```
browser-picker.exe <url>          Show the picker for a URL (what Windows calls)
browser-picker.exe                Open settings (icons, reorder, favourite)
browser-picker.exe --register     Register as a selectable browser (no admin)
browser-picker.exe --unregister   Remove the registration
browser-picker.exe --list         List the browsers that were detected
browser-picker.exe --help         Show help
```

In the picker: click a browser, press **1–9**, press **Enter** for your
favourite/default, or **Esc** to cancel.

Run `browser-picker.exe` with no URL for **settings**: **★** sets the
favourite/default, **▲ ▼** reorder, and the buttons register / set-default.
Order and favourite save instantly (to `HKCU\Software\BrowserPicker`).

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

The GitHub Actions workflow builds the MSVC binary, compiles the Inno Setup
installer, uploads both as artifacts, and attaches them to a GitHub Release when
you push a `v*` tag.

## How detection works

For each of `HKCU\SOFTWARE\Clients\StartMenuInternet`,
`HKLM\SOFTWARE\Clients\StartMenuInternet`, and
`HKLM\SOFTWARE\WOW6432Node\Clients\StartMenuInternet`: enumerate the sub-keys,
read the display name from `Capabilities\ApplicationName` (falling back to the
key's default value), read the launch command from `shell\open\command`, and
parse the executable out of it. Then de-duplicate by exe path and drop Browser
Picker itself. Browsers are launched as `exe <url>`.

## Notes

- The UI is **drawn directly with GDI — no GPU**. No OpenGL, no DirectX, no
  bundled runtime. This is deliberate: it works on any Windows, including Remote
  Desktop sessions and VMs with no graphics adapter (where GPU toolkits fail with
  "no suitable adapter found"). The binary is ~400 KB, double-buffered so it
  doesn't flicker, and there is no background process.
- If the app ever fails to start, it shows the exact error in a dialog and writes
  `%TEMP%\browser-picker-crash.log`.
- The app is DPI-unaware, so Windows scales the window to the correct physical
  size on high-DPI displays (text may be marginally soft at very high scale).

## Roadmap

- Per-domain rules ("always open github.com in Firefox").
- Remember the most recently used browser.

## Credits

- [Browserino](https://github.com/AlexStrNik/Browserino) — native-macOS inspiration.
- [Browserosaurus](https://github.com/will-stone/browserosaurus) &
  [browseratops](https://github.com/riotrah/browseratops) — the original idea and
  the Windows-port attempt.

## License

MIT.
