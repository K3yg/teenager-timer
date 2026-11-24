# Stick Timer (desktop)

A small dial timer that sticks to your screen. Tauri v2: a Rust shell plus the web UI in `ui/index.html`, a port of `../playground-v23.html`.

## How it works

- One transparent, always-on-top window covers the work area of the primary monitor (screen minus taskbar/Dock/menu bar).
- The mouse passes through to whatever is behind it, except over the dial, the panel, the orbit buttons and the tour bubble. The UI reports those shapes, and a Rust poller (30 Hz) toggles click-through when the cursor crosses one.
- Time is counted from the wall clock. The UI repaints only when something visible changes, so it is near 0% CPU when idle.
- Tray / menu bar icon: left click shows or hides the dial. The menu has "Show / hide" and "Quit".
- On macOS the app has no Dock icon, and it shows on every Space and over full-screen apps.
- The first launch starts with an empty task list and a short tour.
- Data (tasks, theme, position) lives in the WebView's localStorage.

## Shortcuts

| | Windows | macOS |
|---|---|---|
| New task (from any app) | Ctrl+Alt+N | ⌘⌥N |
| Open in the center (from any app) | Ctrl+Alt+K | ⌘⌥K |
| Start / pause (from any app) | Ctrl+Alt+Space | ⌘⌥P |
| Opacity when stuck | Ctrl+[ / Ctrl+] | ⌘[ / ⌘] |

With the panel open: ↑/↓ select, Enter start, X done, Esc close.

## Build on Windows

Requirements:
- Rust (rustup, MSVC toolchain)
- Visual Studio 2022 Build Tools with "Desktop development with C++"
- Node 18+

WebView2 ships with Windows 10/11.

```sh
npm ci
npm run build        # -> src-tauri/target/release/bundle/nsis/Stick Timer_<ver>_x64-setup.exe
```

## Build on macOS

```sh
bash scripts/build-mac.sh
```

The script installs what is missing (Xcode Command Line Tools, Rust; Node must already be installed) and builds a universal `.dmg` for Apple Silicon and Intel:
`src-tauri/target/universal-apple-darwin/release/bundle/dmg/`.

The app is ad-hoc signed, with no Apple developer account.
- **Built on your own Mac:** it opens normally.
- **Downloaded `.dmg`:** right-click › Open once, or run:
  `xattr -dr com.apple.quarantine "/Applications/Stick Timer.app"`

`macOSPrivateApi` is on because transparent windows need it. It only rules out the Mac App Store.

## CI

`.github/workflows/build.yml` builds Windows and macOS installers.
- **Run it:** push a tag (`git tag v0.1.0 && git push --tags`) or start it by hand from the Actions tab.
- **Tagged builds:** go to a draft Release.
- **Manual runs:** upload the installers as workflow artifacts.
