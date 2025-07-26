# Sysmon 🛠️
<p align="center">
    <img src="sysmon/images/logo.png" alt="Sysmon Logo" width="120" />
</p>
I sat down one day trying to find a utility monitor I liked for macbooks, and I couldn't. So I made one.

Sysmon is a lightweight, open-source, privacy‑respecting macOS menu‑bar system monitor written in Rust that shows CPU usage, memory usage, and network throughput. 

**Performance**: ~26–33 MB RSS, ~0–1% CPU when idle (brief spikes when menu is open to 3-5%). App size is 754 KB on disk.

![Sysmon Menu Bar](https://img.shields.io/badge/platform-macOS-blue)
![Language](https://img.shields.io/badge/language-Rust-orange)
![License](https://img.shields.io/badge/license-MIT-green)


---

## technical overview

- **Architecture**: Rust + AppKit via `cocoa`/`objc` bindings
- **UI**: `NSStatusItem` manages the menu; `NSTimer` runs sampling while menu is open
- **System data**: `sysinfo` crate provides CPU percentages, memory usage, and network deltas
- **Performance**: Minimal overhead when idle, efficient polling only during active use

---

## Prerequisites

- **macOS** (10.12+ recommended)
- **Xcode Command Line Tools**: `xcode-select --install`
- **Rust toolchain**: Install via [rustup](https://rustup.rs/)

---

## Build & Run

### Super Quick Start

Download sysmon.zip from root dir, unzip, and use! You will need to verify that the app is safe to use in system-settings.

### Quick Start from source

```bash
git clone https://github.com/kushaldudipala/sysmon.git
cd sysmon/sysmon

# Run the app (release build with locked dependencies)
cargo run --release --locked
```

The menu bar will show a new item 🛠️. Click it to see system metrics. Quit via "Quit sysmon".

---

## Create a macOS .app bundle

```bash
cd sysmon/sysmon
cargo build --release
./tools/fetch_sysmon_app.sh
```

**Outputs:**
- `sysmon.app` (next to the crate)
- `../sysmon.zip` (one directory up)

**Note**: For unsigned apps, first run via Control‑click -> Open to bypass Gatekeeper.

---

## Security & Privacy

- **No elevated privileges**: Runs entirely in user space
- **No network access**: All data is read locally from system APIs
- **No persistent storage**: No configuration files or logs written
- **Sandboxed**: Compatible with macOS App Sandbox
- **Memory safe**: Written in Rust with proper error handling

---

## Credits & License

- **App code**: © Kushal Dudipala
- **license**: [MIT License](LICENSE)
- **Icon**: Google's Noto Emoji, Apache License 2.0. License text available in `THIRD_PARTY_LICENSES/NotoEmoji-APACHE-2.0.txt`
- **Dependencies**: Various crates with licenses as per [crates.io](https://crates.io)

---

## Future Expansions

Finding a way to add temp trackers on mac is really tricky without betraying the ideals behind sysmon. To be done next time!

## Support

If you encounter any issues or have questions, please [open an issue](https://github.com/kushaldudipala/sysmon/issues) on GitHub.
