# Sysmon 🛠️
<p align="center">
    <img src="sysmon/images/logo.png" alt="Sysmon Logo" width="120" />
</p>
I sat down one day trying to find a system monitor I liked, and I couldn't. So I made one.

Sysmon is a lightweight, open-source, privacy‑respecting macOS menu‑bar system monitor written in Rust. Shows CPU usage, memory usage, and network throughput. No daemons, no snooping, no hassle.

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

### Quick Start (from source)

```bash
git clone https://github.com/kushaldudipala/sysmon.git
cd sysmon/sysmon

# Run the app (release build with locked dependencies)
cargo run --release --locked
```

The menu bar will show a new item 🛠️. Click it to see system metrics. Quit via "Quit sysmon".

### Create a macOS .app bundle

```bash
cd sysmon/sysmon
./tools/fetch_sysmon_app.sh
```

**Outputs:**
- `sysmon.app` (next to the crate)
- `../sysmon.zip` (one directory up)

**Note**: For unsigned apps, first run via Control‑click → Open to bypass Gatekeeper.

**Expected performance**: ~26–33 MB RSS, ~0–1% CPU when idle (brief spikes when menu is open).

---

## Repository Structure

```
sysmon/
├── src/
│   ├── main.rs              # App entry point + UI logic
│   ├── cocoa_helpers.rs     # AppKit helpers, menu delegate, timer
│   ├── net.rs               # Network sampling via sysinfo
│   ├── ioreport.rs          # Temperature stubs (future work)
│   ├── types.rs             # Main-thread token + retained ObjC wrapper
│   └── units.rs             # Formatting helpers
├── tools/
│   ├── fetch_sysmon_app.sh  # Build + bundle .app
│   ├── make_noto_hat_icon.sh# Build .icns from Noto Emoji
│   └── measure_app.sh       # Live CPU/RSS sampler for info
├── macos/
│   ├── Info.plist           # App bundle metadata
│   ├── sysmon.icns          # Generated app icon
│   └── entitlements.plist   # Sandbox entitlements
├── Cargo.toml               # Project dependencies
└── build.rs                 # Build configuration
```

---

## Dependencies

- **[cocoa](https://crates.io/crates/cocoa)**: macOS AppKit bindings
- **[objc](https://crates.io/crates/objc)**: Objective-C runtime (with exception handling)
- **[sysinfo](https://crates.io/crates/sysinfo)**: Cross-platform system information
- **[once_cell](https://crates.io/crates/once_cell)**: Thread-safe lazy statics
- **[libc](https://crates.io/crates/libc)**: System library bindings

---

## Development

### Building for development

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release --locked
```

### Testing

```bash
# Run unit tests
cargo test

# Check code formatting
cargo fmt --check

# Run clippy lints
cargo clippy
```

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

## Contributing

This program was written in like 2 days by a sleepy undergrad. if you look for long enough, youll find bugs; please let me know!

## Support

If you encounter any issues or have questions, please [open an issue](https://github.com/kushaldudipala/sysmon/issues) on GitHub.