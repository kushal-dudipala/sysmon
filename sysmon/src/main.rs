#![allow(unexpected_cfgs)]
use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSMenuItem, NSStatusBar,
    NSVariableStatusItemLength,
};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use dispatch::Queue;
use objc::{msg_send, sel, sel_impl};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

static SYS: Lazy<Mutex<System>> = Lazy::new(|| {
    let mut s = System::new_with_specifics(
        RefreshKind::new()
            .with_memory(MemoryRefreshKind::everything())
            .with_cpu(CpuRefreshKind::everything()),
    );
    s.refresh_all();
    Mutex::new(s)
});

// Minimal wrapper so we can move these into a thread.
#[derive(Copy, Clone)]
struct ObjcId(*mut std::ffi::c_void);
unsafe impl Send for ObjcId {}
unsafe impl Sync for ObjcId {}

impl ObjcId {
    fn new(ptr: id) -> Self {
        ObjcId(ptr as *mut std::ffi::c_void)
    }
    
    fn as_id(&self) -> id {
        self.0 as id
    }
}

fn main() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

        // Status item
        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item = status_bar.statusItemWithLength_(NSVariableStatusItemLength);
        let button: id = msg_send![status_item, button];
        let title = NSString::alloc(nil).init_str("sysmon …");
        let _: () = msg_send![button, setTitle: title];

        // Menu with a single, updatable item
        let menu = NSMenu::new(nil).autorelease();
        let item = NSMenuItem::new(nil).autorelease();
        let _: () = msg_send![item, setTitle: NSString::alloc(nil).init_str("Loading…")];
        let _: () = msg_send![menu, addItem: item];
        let _: () = msg_send![status_item, setMenu: menu];

        let item_ptr = ObjcId::new(item);
        let button_ptr = ObjcId::new(button);

        // Worker thread computes metrics
        thread::spawn(move || loop {
            let (cpu, used_gb, total_gb) = sample();
            let title = format!("CPU {:>4.1}%  MEM {:>4.1}/{:>4.1}G", cpu, used_gb, total_gb);
            let details = format!(
                "CPU:  {:.1}%\nMem:  {:.1}/{:.1} GB",
                cpu, used_gb, total_gb
            );

            // Post UI update back to main thread
            Queue::main().exec_sync(move || {
                let ns_title = NSString::alloc(nil).init_str(&title);
                let ns_details = NSString::alloc(nil).init_str(&details);
                let _: () = msg_send![button_ptr.as_id(), setTitle: ns_title];
                let _: () = msg_send![item_ptr.as_id(), setTitle: ns_details]; 
            });
            thread::sleep(Duration::from_secs(1));
        });

        app.run();
    }
}

fn sample() -> (f32, f32, f32) {
    let mut sys = SYS.lock().unwrap();
    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu = sys.global_cpu_info().cpu_usage();
    let used_gib  = bytes_to_gib(sys.used_memory());
    let total_gib = bytes_to_gib(sys.total_memory());
    (cpu, used_gib, total_gib)
}

fn bytes_to_gib(bytes: u64) -> f32 {
    // round to 1 decimal place
    let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    (gib * 10.0).round() as f32 / 10.0
}
