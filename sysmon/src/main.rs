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
use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;

use once_cell::sync::Lazy;
use std::backtrace::Backtrace;
use std::cell::RefCell;
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

/* ---------------- Global sysinfo cache ---------------- */

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

/* ---------------- UI state we mutate on menu open ---------------- */

struct UiState {
    button: UiObj,
    cpu_item: UiObj,
    mem_item: UiObj,
    _delegate: UiObj, // keep it alive
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

/* ---------------- Refresh logic called from delegate ---------------- */

fn refresh_on_click() {
    let mt = MainThreadToken::acquire();
    with_ui_state(|ui| {
        let (cpu, used_gb, total_gb) = sample();

        set_menu_item_title(&mt, &ui.cpu_item, &format!("CPU:  {:.1}%", cpu));
        set_menu_item_title(
            &mt,
            &ui.mem_item,
            &format!("Mem:  {:.1}/{:.1} GB", used_gb, total_gb),
        );
        // The status button title stays as 🧪
    });
}

/* ---------------- main ---------------- */

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

        // Menubar emoji
        set_button_title(&mt, &button, "🧪");

        // Build menu
        let menu_id: id = NSMenu::new(nil).autorelease();
        let menu = UiObj::from_raw_retained(menu_id);

        // One line per metric
        let cpu_item = new_menu_item_with_title(&mt, "CPU:  …");
        let mem_item = new_menu_item_with_title(&mt, "Mem:  …");

        menu_add_item(&mt, &menu, &cpu_item);
        menu_add_item(&mt, &menu, &mem_item);

        // Quit
        let quit_item = make_quit_menu_item(&mt, "Quit sysmon");
        menu_add_item(&mt, &menu, &quit_item);

        // Attach delegate that refreshes when menu opens
        set_refresh_callback(refresh_on_click);
        let delegate = attach_menu_delegate(&mt, &menu);

        // Set menu on the status item
        let status_item_ptr = UiObj::from_raw_retained(status_item);
        status_item_set_menu(&mt, &status_item_ptr, &menu);

        set_ui_state(UiState {
            button,
            cpu_item,
            mem_item,
            _delegate: delegate,
        });

        app.run();
    }
}

/* ---------------- sampling ---------------- */

fn sample() -> (f32, f32, f32) {
    let mut sys = lock_sys();

    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu = sys.global_cpu_info().cpu_usage();
    let used_gb = bytes_to_gb(sys.used_memory());   // you asked to ignore the unit bug
    let total_gb = bytes_to_gb(sys.total_memory());
    (cpu, used_gb, total_gb)
}
