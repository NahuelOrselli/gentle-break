# Installing Gentle Break

Gentle Break is a desktop app (Tauri 2): a floating clock, alarms, breaks, and a settings panel. End users only need an installer; Node and Rust are required to build from source.

## macOS

**Requirements:** Apple Silicon Mac (M1 or later) and macOS 12 or later. CI builds `aarch64`; Intel Macs need a local build with target `x86_64-apple-darwin`.

### From a release

1. Download the `.dmg` from the [latest release](https://github.com/NahuelOrselli/gentle-break/releases/latest) (name like `Gentle Break_0.1.0_aarch64.dmg`).
2. Open the DMG and drag **Gentle Break** into **Applications**.
3. Eject the disk image.

### First open (unsigned app)

macOS may block the first launch:

- **macOS 15 (Sequoia) and later:** open the app, dismiss the alert, go to **System Settings → Privacy & Security → Security**, and click **Open Anyway** next to the Gentle Break message.
- **macOS 12–14:** Control-click the app → **Open** → confirm.

From Terminal (only if you trust the file source):

```sh
xattr -dr com.apple.quarantine "/Applications/Gentle Break.app"
```

### Menu bar and widget

- Drag the floating clock to move it; click to open the panel.
- **Menu bar:** Open, Hide/Show widget, Do not disturb, Quit.
- In **Settings** you can hide the widget and keep using the app from the menu bar.

### Where data lives

| What | Where |
| --- | --- |
| Settings, alarms, breaks, sounds | `~/Library/Application Support/dev.gentlebreak.desktop/` (`state.json`, `sounds/` folder) |
| Widget position | same `state.json` |

### Uninstall

1. Quit Gentle Break from the menu bar (**Quit**).
2. Move `/Applications/Gentle Break.app` to the Trash.
3. Optional: delete `~/Library/Application Support/dev.gentlebreak.desktop/`.

---

## Windows

Download the NSIS `.exe` from the release for `x86_64-pc-windows-msvc` and run the installer.

Local build (on Windows):

```sh
npm ci
npm run package
```

Output: `src-tauri/target/release/bundle/nsis/`.

---

## Linux

Download the `.deb` from the release (`x86_64-unknown-linux-gnu`) and install with your package manager, for example:

```sh
sudo dpkg -i gentle-break_*.deb
```

Local build (Debian/Ubuntu; GTK/WebKit deps):

```sh
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf build-essential
npm ci
npm run package
```

---

## Build from source

```sh
npm ci
npm run desktop          # development
npm run package:mac      # macOS: .app + .dmg
npm run package          # dmg / exe / deb depending on OS
```

Backend tests:

```sh
cd src-tauri && cargo test --lib
```

---

## Publishing a release (maintainers)

1. Bump the version in `package.json` and `src-tauri/tauri.conf.json` (same semver, e.g. `0.1.0`).
2. Commit and tag:
   ```sh
   git tag v0.1.0
   git push origin main
   git push origin v0.1.0
   ```
3. The **Desktop** workflow on GitHub Actions builds macOS, Windows, and Linux and, on `v*` tags, uploads installers to the [GitHub Release](https://github.com/NahuelOrselli/gentle-break/releases) for that tag.

You can also run **Actions → Desktop → Run workflow** manually (artifacts only, no release, unless the run is on a tag).
