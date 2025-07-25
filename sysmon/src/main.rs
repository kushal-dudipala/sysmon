#![deny(unsafe_op_in_unsafe_fn)]

mod cocoa_helpers;
mod types;
mod units;

use crate::cocoa_helpers::*;
use crate::types::UiObj;
use units::bytes_to_gb;

use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSStatusBar,
    NSVariableStatusItemLength,
};
use cocoa::base::{id, nil, YES};
use cocoa::foundation::NSAutoreleasePool;

use objc::{class, msg_send, sel, sel_impl};

use block::ConcreteBlock;
use once_cell::sync::Lazy;
use std::backtrace::Backtrace;
use std::panic;
use std::process;
use std::sync::{Mutex, MutexGuard};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

#[derive(Clone)]
struct UiHandles {
    button: UiObj,
    item: UiObj,
}

/* ---------------- Debug-time guard to ensure UI code stays on main thread ---------------- */

extern "C" {
    fn pthread_main_np() -> libc::c_int;
}

#[inline(always)]
fn assert_main_thread() {
    unsafe {
        if pthread_main_np() == 0 {
            eprintln!("UI code touched off the main thread!");
            process::abort();
        }
    }
}

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
        // Never unwind across Obj-C. Abort hard.
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

/// Lock the global `System`, recovering from a poisoned mutex by
/// reinitializing the inner `System` and returning a valid guard.
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

    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        // Status item + button
        let status_bar: id = NSStatusBar::systemStatusBar(nil);
        let status_item: id = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

        let raw_button: id = status_button(status_item);
        debug_assert!(!raw_button.is_null(), "status_button returned null");
        let button = UiObj::from_raw(raw_button);

        set_button_title(&button, "sysmon");

        // Menu + first line
        let item = new_menu_item_with_title("Loading…");

        let menu_id: id = NSMenu::new(nil).autorelease();
        let menu = UiObj::from_raw(menu_id);
        menu_add_item(&menu, &item);

        let status_item_ptr = UiObj::from_raw(status_item);
        status_item_set_menu(&status_item_ptr, &menu);

        let ui = UiHandles { button, item };

        // ---- Schedule a repeating NSTimer (main thread) ----
        // macOS 10.12+: +[NSTimer scheduledTimerWithTimeInterval:repeats:block:]
        let tick = ConcreteBlock::new(move |_: id| {
            assert_main_thread();
            // Make sure a panic inside this block doesn't unwind into Obj‑C:
            let _ = std::panic::catch_unwind(|| {
                let _pool = NSAutoreleasePool::new(nil);

                let (cpu, used_gb, total_gb) = sample();
                let title =
                    format!("CPU {:>4.1}%  MEM {:>4.1}/{:>4.1}G", cpu, used_gb, total_gb);
                let details =
                    format!("CPU:  {:.1}%\nMem:  {:.1}/{:.1} GB", cpu, used_gb, total_gb);

                set_button_title(&ui.button, &title);
                set_menu_item_title(&ui.item, &details);
            });
        })
        .copy(); // Move to heap; NSTimer retains it

        let interval: f64 = 1.0;
        let _: id = msg_send![class!(NSTimer),
            scheduledTimerWithTimeInterval: interval
            repeats: YES
            block: &*tick
        ];

        app.run();
    }
}

/* ---------------- sampling (pure Rust/sysinfo) ---------------- */

fn sample() -> (f32, f32, f32) {
    let mut sys = lock_sys();

    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu = sys.global_cpu_info().cpu_usage();
    let used_gib = bytes_to_gb(sys.used_memory());
    let total_gib = bytes_to_gb(sys.total_memory());
    (cpu, used_gib, total_gib)
}
