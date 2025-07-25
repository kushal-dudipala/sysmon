use cocoa::base::{id, nil, YES};
use cocoa::foundation::{NSAutoreleasePool, NSRunLoop, NSString};
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use once_cell::sync::{Lazy, OnceCell};
use std::cell::RefCell;
use cocoa::appkit::NSMenuItem;
use crate::types::{MainThreadToken, UiObj};

/* ---------------- Refresh callback plumbing ---------------- */

type RefreshFn = fn(bool);
static REFRESH_CB: OnceCell<RefreshFn> = OnceCell::new();
pub fn set_refresh_callback(cb: RefreshFn) { let _ = REFRESH_CB.set(cb); }

/* ---------------- Basic UI helpers ---------------- */

pub fn status_button(_mt: &MainThreadToken, status_item: id) -> id {
    unsafe { msg_send![status_item, button] }
}

// set_button_title
pub fn set_button_title(_mt: &MainThreadToken, ptr: &UiObj, title: &str) {
    unsafe {
        let s = NSString::alloc(nil).init_str(title).autorelease();
        let _: () = msg_send![ptr.as_id(), setTitle: s];
    }
}


// set_menu_item_title
pub fn set_menu_item_title(_mt: &MainThreadToken, ptr: &UiObj, title: &str) {
    unsafe {
        let s = NSString::alloc(nil).init_str(title).autorelease();
        let _: () = msg_send![ptr.as_id(), setTitle: s];
    }
}


// new_menu_item_with_title
pub fn new_menu_item_with_title(_mt: &MainThreadToken, title: &str) -> UiObj {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let item: id = NSMenuItem::new(nil).autorelease();
        assert!(!item.is_null());
        let s = NSString::alloc(nil).init_str(title).autorelease();
        let _: () = msg_send![item, setTitle: s];
        let _: () = msg_send![item, setEnabled: YES];
        UiObj::from_raw_retained(item)
    }
}

pub fn menu_add_item(_mt: &MainThreadToken, menu: &UiObj, item: &UiObj) {
    unsafe { let _: () = msg_send![menu.as_id(), addItem: item.as_id()]; }
}

pub fn status_item_set_menu(_mt: &MainThreadToken, status_item: &UiObj, menu: &UiObj) {
    unsafe { let _: () = msg_send![status_item.as_id(), setMenu: menu.as_id()]; }
}

/* ---------------- NSMenuDelegate using NSTimer (in proper modes) ---------------- */

thread_local! {
    static TIMER: RefCell<id> = RefCell::new(nil);
}

extern "C" fn timer_fired(_this: &Object, _cmd: Sel, _user_info: id) {
    if let Some(cb) = REFRESH_CB.get() { cb(true); }
}

extern "C" fn menu_will_open(this: &mut Object, _cmd: Sel, _menu: id) {
    // Kick one immediate refresh.
    if let Some(cb) = REFRESH_CB.get() { cb(true); }

    unsafe {
        // 1) Create an *unscheduled* repeating timer.
        let interval: f64 = 1.0;
        let timer: id = msg_send![class!(NSTimer),
            timerWithTimeInterval: interval
            target: this
            selector: sel!(timerFired:)
            userInfo: nil
            repeats: YES
        ];

        // Optional: relax timing a bit to save power.
        let _: () = msg_send![timer, setTolerance: 0.2_f64];

        // 2) Add the timer to run loop in modes that stay active during menu tracking.
        let run_loop: id = NSRunLoop::currentRunLoop();
        let common_mode = NSString::alloc(nil).init_str("NSRunLoopCommonModes").autorelease();
        let track_mode  = NSString::alloc(nil).init_str("NSEventTrackingRunLoopMode").autorelease();


        let _: () = msg_send![run_loop, addTimer: timer forMode: common_mode];
        let _: () = msg_send![run_loop, addTimer: timer forMode: track_mode];

        TIMER.with(|slot| slot.replace(timer));
    }
}

extern "C" fn menu_did_close(_this: &mut Object, _cmd: Sel, _menu: id) {
    // Let the app know we closed (handy if you ever want to stop refreshing).
    if let Some(cb) = REFRESH_CB.get() { cb(false); }

    // Invalidate and drop the timer.
    TIMER.with(|slot| {
        let t = *slot.borrow();
        if t != nil {
            unsafe { let _: () = msg_send![t, invalidate]; }
            slot.replace(nil);
        }
    });
}

static MENU_DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SysmonMenuDelegate", superclass)
        .expect("Unable to declare delegate");

    decl.add_method(sel!(menuWillOpen:),  menu_will_open  as extern "C" fn(&mut Object, Sel, id));
    decl.add_method(sel!(menuDidClose:),  menu_did_close  as extern "C" fn(&mut Object, Sel, id));
    decl.add_method(sel!(timerFired:),    timer_fired     as extern "C" fn(&Object, Sel, id));

    decl.register()
});

pub fn attach_menu_delegate(_mt: &MainThreadToken, menu: &UiObj) -> UiObj {
    unsafe {
        let delegate: id = msg_send![*MENU_DELEGATE_CLASS, new];
        let _: () = msg_send![menu.as_id(), setDelegate: delegate];
        UiObj::from_raw_retained(delegate)
    }
}

/* ---------------- Quit menu (target + selector) ---------------- */
extern "C" fn quit_now(this: &Object, _cmd: Sel, _sender: id) {
    // Terminate via AppKit (clean and fast).
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, terminate: this];
    }
}

static QUIT_CLASS: Lazy<&'static Class> = Lazy::new(|| {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SysmonQuitTarget", superclass)
        .expect("Unable to declare SysmonQuitTarget");

    // Only this part requires unsafe.
    unsafe {
        decl.add_method(sel!(quitNow:), quit_now as extern "C" fn(&Object, Sel, id));
    }

    decl.register()
});

fn make_quit_target() -> id {
    unsafe { msg_send![*QUIT_CLASS, new] }
}

// make_quit_menu_item
pub fn make_quit_menu_item(_mt: &MainThreadToken, title: &str) -> (UiObj, UiObj) {
    unsafe {
        let title_ns = NSString::alloc(nil).init_str(title).autorelease();
        let key_equiv = NSString::alloc(nil).init_str("").autorelease();
        let item: id = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(title_ns, sel!(quitNow:), key_equiv)
            .autorelease();
        let target = make_quit_target();
        let _: () = msg_send![item, setTarget: target];
        let _: () = msg_send![item, setEnabled: YES];
        (UiObj::from_raw_retained(item), UiObj::from_raw_retained(target))
    }
}
