#![deny(unsafe_op_in_unsafe_fn)]

mod cocoa_helpers;
mod types;

use crate::cocoa_helpers::*;
use crate::types::SendUiPtr;
use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSStatusBar,
    NSVariableStatusItemLength,
};
use cocoa::base::nil;
use cocoa::foundation::NSAutoreleasePool;
use dispatch::Queue;
use objc::rc::StrongPtr;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use objc::runtime::Object;


#[derive(Clone)]
struct UiHandles {
    button: SendUiPtr,
    item: SendUiPtr,
}

// SAFETY: Only used on main thread
unsafe impl Send for UiHandles {}
unsafe impl Sync for UiHandles {}

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

        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

        let raw_button: *mut Object = status_button(status_item);
        debug_assert!(!raw_button.is_null(), "button ptr is null");
        let button = SendUiPtr::new(raw_button).unwrap();

        set_button_title(&button, "sysmon …");

        let item = new_menu_item_with_title("Loading…");
        assert!(
            !item.as_ptr().is_null(),
            "Menu item returned null"
        );

        let menu = NSMenu::new(nil).autorelease();
        let menu = SendUiPtr::new(menu).expect("menu ptr is null");
        menu_add_item(&menu, &item);
        let status_item_ptr = SendUiPtr::new(status_item).expect("status item ptr is null");
        status_item_set_menu(&status_item_ptr, &menu);

        let _button_sp = StrongPtr::retain(button.as_ptr());
        let _item_sp   = StrongPtr::retain(item.as_ptr());

        let handles = UiHandles {
            button,
            item,
        };

        thread::spawn(move || loop {
            let (cpu, used_gb, total_gb) = sample();
            let title = format!("CPU {:>4.1}%  MEM {:>4.1}/{:>4.1}G", cpu, used_gb, total_gb);
            let details = format!("CPU:  {:.1}%\nMem:  {:.1}/{:.1} GB", cpu, used_gb, total_gb);

            let h = handles.clone();
            Queue::main().exec_async(move || {
                let _pool = NSAutoreleasePool::new(nil);
                set_button_title(&h.button, &title);
                set_menu_item_title(&h.item, &details);
            });

            thread::sleep(Duration::from_secs(1));
        });

        app.run();
    }
}

fn sample() -> (f32, f32, f32) {
    let mut sys = match SYS.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu = sys.global_cpu_info().cpu_usage();
    let used_gib = kib_to_gib(sys.used_memory());
    let total_gib = kib_to_gib(sys.total_memory());
    (cpu, used_gib, total_gib)
}

fn kib_to_gib(kib: u64) -> f32 {
    let gib = kib as f64 / (1024.0 * 1024.0);
    ((gib * 10.0).round() / 10.0) as f32
}
