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
use std::sync::Mutex;
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
            std::process::abort();
        }
    }
}

/* ---------------- Global sysinfo cache (pure data, Send + Sync OK) ---------------- */

static SYS: Lazy<Mutex<System>> = Lazy::new(|| {
    let mut s = System::new_with_specifics(
        RefreshKind::new()
            .with_memory(MemoryRefreshKind::everything())
            .with_cpu(CpuRefreshKind::everything()),
    );
    s.refresh_all();
    Mutex::new(s)
});

fn main() {
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

        // No extra StrongPtr::retain needed; UiObj owns & retains.

        let ui = UiHandles { button, item };

        // ---- Schedule a repeating NSTimer (main thread) ----
        // macOS 10.12+: +[NSTimer scheduledTimerWithTimeInterval:repeats:block:]
        let tick = ConcreteBlock::new(move |_: id| {
            assert_main_thread();
            let _pool = NSAutoreleasePool::new(nil);

            let (cpu, used_gb, total_gb) = sample();
            let title = format!("CPU {:>4.1}%  MEM {:>4.1}/{:>4.1}G", cpu, used_gb, total_gb);
            let details = format!("CPU:  {:.1}%\nMem:  {:.1}/{:.1} GB", cpu, used_gb, total_gb);

            set_button_title(&ui.button, &title);
            set_menu_item_title(&ui.item, &details);
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
    let mut sys = match SYS.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu = sys.global_cpu_info().cpu_usage();
    let used_gib = bytes_to_gb(sys.used_memory());
    let total_gib = bytes_to_gb(sys.total_memory());
    (cpu, used_gib, total_gib)
}
