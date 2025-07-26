#![deny(unsafe_op_in_unsafe_fn)]

mod cocoa_helpers;
mod ioreport;
mod types;
mod units;
mod net;

use crate::cocoa_helpers::*;
use crate::types::{MainThreadToken, UiObj};
use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSStatusBar,
    NSVariableStatusItemLength,
};
use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;

use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::panic;
use std::sync::{Mutex, MutexGuard};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use units::{bytes_to_gb, fmt_rate};

/* ---------------- Panic hook (abort-fast on panic) ---------------- */

fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        eprintln!("========= sysmon PANIC =========");
        if let Some(loc) = info.location() {
            eprintln!("Location: {}:{}", loc.file(), loc.line());
        }
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            eprintln!("Payload: {}", s);
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            eprintln!("Payload: {}", s);
        } else {
            eprintln!("Payload: <non-string panic payload>");
        }
        eprintln!("=================================");
        std::process::abort();
    }));
}

fn abort_if_dyld_injection() {
    use std::env;

    const BLOCK: &[&str] = &[
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FORCE_FLAT_NAMESPACE",
        "LD_PRELOAD",
    ];

    let allow = env::var_os("SYSMON_ALLOW_UNSAFE_LAUNCH").is_some();

    let mut offenders = Vec::new();
    for (k, v) in env::vars() {
        if BLOCK.contains(&k.as_str()) && !v.is_empty() {
            offenders.push(format!("{k}={v}"));
        }
    }

    if !offenders.is_empty() && !allow {
        eprintln!(
            "Refusing to run due to high‑risk injection env:\n  {}\
             \nSet SYSMON_ALLOW_UNSAFE_LAUNCH=1 to bypass (not recommended).",
            offenders.join("\n  ")
        );
        std::process::exit(78);
    }

    // Debug-only note for benign DYLD_* (e.g., DYLD_FALLBACK_LIBRARY_PATH)
    #[cfg(debug_assertions)]
    for (k, _) in env::vars() {
        if k.starts_with("DYLD_") && !BLOCK.contains(&k.as_str()) {
            eprintln!("Note: {k} is set (continuing).");
        }
    }
}


#[inline]
unsafe fn nonnull(obj: cocoa::base::id, what: &str) -> cocoa::base::id {
    if obj == cocoa::base::nil {
        eprintln!("{what} returned nil; aborting.");
        std::process::abort();
    }
    obj
}


/* ---------------- Global sysinfo cache ---------------- */

static SYS: Lazy<Mutex<System>> = Lazy::new(|| {
    let s = new_system();
    Mutex::new(s)
});

fn new_system() -> System {
    let mut s = System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::everything())
            .with_cpu(CpuRefreshKind::everything()),
    );
    s.refresh_all();
    s
}

fn lock_sys() -> MutexGuard<'static, System> {
    match SYS.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("SYS mutex was poisoned; repairing and continuing.");
            let mut guard = poisoned.into_inner();
            *guard = new_system();
            guard
        }
    }
}

/* ---------------- UI state ---------------- */

struct UiState {
    _button: UiObj,
    cpu_item: UiObj,
    mem_item: UiObj,
    cpu_t_item: UiObj,
    gpu_t_item: UiObj,
    net_item: UiObj,
    _delegate: UiObj,     // retain the NSTimer menu delegate
    _quit_target: UiObj,  // retain the quit target so selector stays valid
}

thread_local! {
    static UI: RefCell<Option<UiState>> = RefCell::new(None);
}

fn set_ui_state(state: UiState) {
    UI.with(|slot| *slot.borrow_mut() = Some(state));
}

fn with_ui_state<F: FnOnce(&UiState)>(f: F) {
    UI.with(|slot| {
        if let Some(ref ui) = *slot.borrow() {
            f(ui);
        }
    });
}

/* ---------------- Refresh (timer fires while menu is open) ---------------- */

fn refresh(_opened: bool) {
    let mt = MainThreadToken::acquire();
    with_ui_state(|ui| {
        let (cpu, used_gb, total_gb) = sample();
        let cpu_t = ioreport::cpu_temp_c();
        let gpu_t = ioreport::gpu_temp_c();
        let (rx_bps, tx_bps) = net::net_usage_bps();

        set_menu_item_title(&mt, &ui.cpu_item, &format!("CPU:   {:.1}%", cpu));
        set_menu_item_title(&mt, &ui.mem_item, &format!("Mem:   {:.1}/{:.1} GB", used_gb, total_gb));
        set_menu_item_title(
            &mt,
            &ui.cpu_t_item,
            &format!(
                "CPU T: {}",
                cpu_t.map(|t| format!("{t:.1} °C")).unwrap_or_else(|| "—".into())
            ),
        );
        set_menu_item_title(
            &mt,
            &ui.gpu_t_item,
            &format!(
                "GPU T: {}",
                gpu_t.map(|t| format!("{t:.1} °C")).unwrap_or_else(|| "—".into())
            ),
        );
        set_menu_item_title(
            &mt,
            &ui.net_item,
            &format!("Net:   ↑{} ↓{}", fmt_rate(tx_bps), fmt_rate(rx_bps)),
        );
    });
}

/* ---------------- main ---------------- */

fn main() {
    abort_if_dyld_injection();
    install_panic_hook();

    let mt = MainThreadToken::acquire();

    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        // Status item + button
        let status_bar: id = NSStatusBar::systemStatusBar(nil);
        let status_item: id = nonnull(
            status_bar.statusItemWithLength_(NSVariableStatusItemLength),
            "NSStatusBar::statusItemWithLength_",
        );

        let raw_button: id = nonnull(status_button(&mt, status_item), "status_button");
        let button = UiObj::from_raw_retained(raw_button);

        // Menubar emoji
        set_button_title(&mt, &button, "🛠️");

        // Build menu
        let menu_id: id = nonnull(NSMenu::new(nil).autorelease(), "NSMenu::new");
        let menu = UiObj::from_raw_retained(menu_id);

        // One line per metric
        let cpu_item = new_menu_item_with_title(&mt, "CPU:   …");
        let mem_item = new_menu_item_with_title(&mt, "Mem:   …");
        let cpu_t_item = new_menu_item_with_title(&mt, "CPU T: …");
        let gpu_t_item = new_menu_item_with_title(&mt, "GPU T: …");
        let net_item = new_menu_item_with_title(&mt, "Net:   …");

        menu_add_item(&mt, &menu, &cpu_item);
        menu_add_item(&mt, &menu, &mem_item);
        menu_add_item(&mt, &menu, &cpu_t_item);
        menu_add_item(&mt, &menu, &gpu_t_item);
        menu_add_item(&mt, &menu, &net_item);

        // Quit (single, canonical quit path; target retained in UiState)
        let (quit_item, quit_target) = make_quit_menu_item(&mt, "Quit sysmon");
        menu_add_item(&mt, &menu, &quit_item);

        // Attach NSTimer-based delegate that refreshes while the menu is open
        set_refresh_callback(refresh);
        let delegate = attach_menu_delegate(&mt, &menu);

        // Set menu on the status item
        let status_item_ptr = UiObj::from_raw_retained(status_item);
        status_item_set_menu(&mt, &status_item_ptr, &menu);

        set_ui_state(UiState {
            _button: button,
            cpu_item,
            mem_item,
            cpu_t_item,
            gpu_t_item,
            net_item,
            _delegate: delegate,
            _quit_target: quit_target,
        });

        app.run();
    }
}

/* ---------------- sampling ---------------- */

fn sample() -> (f32, f32, f32) {
    let mut sys = lock_sys();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpu = sys.global_cpu_usage();
    let used_gb = bytes_to_gb(sys.used_memory());
    let total_gb = bytes_to_gb(sys.total_memory());

    (cpu, used_gb, total_gb)
}