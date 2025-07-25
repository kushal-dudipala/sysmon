#![deny(unsafe_op_in_unsafe_fn)]

mod cocoa_helpers;
mod types;
mod units;

use crate::cocoa_helpers::*;
use crate::types::{MainThreadToken, UiObj};
use units::bytes_to_gb;

use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSStatusBar,
    NSVariableStatusItemLength,
};
use cocoa::base::{id, nil, YES};
use cocoa::foundation::NSAutoreleasePool;

use block::ConcreteBlock;
use objc::{class, msg_send, sel, sel_impl};

use once_cell::sync::Lazy;
use std::backtrace::Backtrace;
use std::panic;
use std::sync::{Mutex, MutexGuard};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/* ---------------- Panic hook ---------------- */

fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let bt = Backtrace::force_capture();
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
        eprintln!("Backtrace:\n{bt}");
        eprintln!("=================================");
        unsafe { libc::abort() }
    }));
}

/* ---------------- Global sysinfo cache (pure data, Send + Sync OK) ---------------- */

static SYS: Lazy<Mutex<System>> = Lazy::new(|| {
    let s = new_system();
    Mutex::new(s)
});

fn new_system() -> System {
    let mut s = System::new_with_specifics(
        RefreshKind::new()
            .with_memory(MemoryRefreshKind::everything())
            .with_cpu(CpuRefreshKind::everything()),
    );
    s.refresh_all();
    s
}

/// Recover from poisoned mutexes by rebuilding the System.
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

fn main() {
    install_panic_hook();

    let mt = MainThreadToken::acquire();

    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        // Status item + button
        let status_bar: id = NSStatusBar::systemStatusBar(nil);
        let status_item: id = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

        let raw_button: id = status_button(&mt, status_item);
        debug_assert!(!raw_button.is_null(), "status_button returned null");
        let button = UiObj::from_raw_retained(raw_button);

        set_button_title(&mt, &button, "sysmon");

        // Menu + first line (live metrics)
        let item = new_menu_item_with_title(&mt, "Loading…");

        // ---- Schedule a repeating NSTimer (main thread) ----
        let ui_button = button.clone_retained();
        let ui_item = item.clone_retained();

        let update_ui = move || {
            let (cpu, used_gb, total_gb) = sample();
            let title = format!("CPU {:>4.1}%  MEM {:>4.1}/{:>4.1}G", cpu, used_gb, total_gb);
            let details = format!("CPU:  {:.1}%\nMem:  {:.1}/{:.1} GB", cpu, used_gb, total_gb);
            let mt = MainThreadToken::acquire();
            set_button_title(&mt, &ui_button, &title);
            set_menu_item_title(&mt, &ui_item, &details);
        };

        let tick = ConcreteBlock::new(move |_: id| {
            let _pool = NSAutoreleasePool::new(nil);

            #[cfg(panic = "unwind")]
            {
                let _ = std::panic::catch_unwind(|| update_ui());
            }
            #[cfg(panic = "abort")]
            {
                update_ui();
            }
        })
        .copy(); // Move to heap; NSTimer retains it

        let interval: f64 = 1.0;
        let timer: id = msg_send![class!(NSTimer),
            scheduledTimerWithTimeInterval: interval
            repeats: YES
            block: &*tick
        ];

        // Build menu & add items
        let menu_id: id = NSMenu::new(nil).autorelease();
        let menu = UiObj::from_raw_retained(menu_id);
        menu_add_item(&mt, &menu, &item);

        // Quit item (graceful)
        let quit_item = make_quit_menu_item(&mt, "Quit sysmon", timer);
        menu_add_item(&mt, &menu, &quit_item);

        let status_item_ptr = UiObj::from_raw_retained(status_item);
        status_item_set_menu(&mt, &status_item_ptr, &menu);

        app.run();
    }
}

/* ---------------- sampling (pure Rust/sysinfo) ---------------- */

fn sample() -> (f32, f32, f32) {
    let mut sys = lock_sys();

    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu = sys.global_cpu_info().cpu_usage();
    let used_gb = bytes_to_gb(sys.used_memory());   // you said ignore the unit bug
    let total_gb = bytes_to_gb(sys.total_memory());
    (cpu, used_gb, total_gb)
}
