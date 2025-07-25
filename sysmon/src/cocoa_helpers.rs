// src/cocoa_helpers.rs

use cocoa::appkit::NSMenuItem;
use cocoa::base::{id, nil, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use once_cell::sync::{Lazy, OnceCell};
use std::cell::RefCell;

use block::ConcreteBlock;

use crate::types::{MainThreadToken, UiObj};

/* ---------------- Refresh callback plumbing ---------------- */

/// `opened = true` while menu is open; `false` when it closes.
type RefreshFn = fn(bool);

static REFRESH_CB: OnceCell<RefreshFn> = OnceCell::new();

pub fn set_refresh_callback(cb: RefreshFn) {
    let _ = REFRESH_CB.set(cb);
}

/* ---------------- Basic UI helpers (all require &MainThreadToken) ---------------- */

pub fn status_button(_mt: &MainThreadToken, status_item: id) -> id {
    unsafe { msg_send![status_item, button] }
}

pub fn set_button_title(_mt: &MainThreadToken, ptr: &UiObj, title: &str) {
    unsafe {
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![ptr.as_id(), setTitle: title_str];
    }
}

pub fn set_menu_item_title(_mt: &MainThreadToken, ptr: &UiObj, title: &str) {
    unsafe {
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![ptr.as_id(), setTitle: title_str];
    }
}

pub fn new_menu_item_with_title(_mt: &MainThreadToken, title: &str) -> UiObj {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let item: id = NSMenuItem::new(nil).autorelease();
        assert!(!item.is_null(), "NSMenuItem::new returned null");
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![item, setTitle: title_str];
        let _: () = msg_send![item, setEnabled: YES];
        UiObj::from_raw_retained(item)
    }
}

pub fn menu_add_item(_mt: &MainThreadToken, menu: &UiObj, item: &UiObj) {
    unsafe {
        let _: () = msg_send![menu.as_id(), addItem: item.as_id()];
    }
}

pub fn status_item_set_menu(_mt: &MainThreadToken, status_item: &UiObj, menu: &UiObj) {
    unsafe {
        let _: () = msg_send![status_item.as_id(), setMenu: menu.as_id()];
    }
}

/* ---------------- Quit menu (target + selector) ---------------- */

extern "C" fn quit_now(this: &Object, _cmd: Sel, _sender: id) {
    // Ask AppKit to terminate cleanly (will be fast since we have no background thread).
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, terminate: this];
    }
}

static QUIT_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SysmonQuitTarget", superclass)
        .expect("Unable to declare SysmonQuitTarget");

    // NOTE: add_method is unsafe per objc crate.
    unsafe {
        decl.add_method(
            sel!(quitNow:),
            quit_now as extern "C" fn(&Object, Sel, id),
        );
    }

    decl.register()
});

fn make_quit_target() -> id {
    unsafe { msg_send![*QUIT_CLASS, new] }
}

/// Returns (menu_item, target). Keep `target` retained by storing it in your UiState.
pub fn make_quit_menu_item(_mt: &MainThreadToken, title: &str) -> (UiObj, UiObj) {
    unsafe {
        let title_ns = NSString::alloc(nil).init_str(title);
        let key_equiv = NSString::alloc(nil).init_str("");
        let item: id = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(title_ns, sel!(quitNow:), key_equiv)
            .autorelease();

        let target = make_quit_target();
        let _: () = msg_send![item, setTarget: target];
        let _: () = msg_send![item, setEnabled: YES];

        (UiObj::from_raw_retained(item), UiObj::from_raw_retained(target))
    }
}

/* ---------------- NSMenuDelegate using NSTimer (block-based) ---------------- */

thread_local! {
    // Store the NSTimer for this menu so we can invalidate it immediately on close.
    static TIMER: RefCell<id> = RefCell::new(nil);
}

extern "C" fn menu_will_open(_this: &mut Object, _cmd: Sel, _menu: id) {
    // Do an immediate refresh.
    if let Some(cb) = REFRESH_CB.get() {
        cb(true);
    }

    // Create a repeating NSTimer with a block that calls our refresh callback.
    unsafe {
        // Copy block to heap before passing to Obj-C (as recommended by the `block` crate).
        // When the timer is invalidated, AppKit will release the retained block.
        let blk = ConcreteBlock::new(move |_: id| {
            if let Some(cb) = REFRESH_CB.get() {
                cb(true);
            }
        })
        .copy();

        // +scheduledTimerWithTimeInterval:repeats:block: schedules itself on current run loop.
        // Ref: Apple docs.
        let interval: f64 = 1.0;
        let timer: id = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: interval
            repeats: YES
            block: &*blk
        ];

        TIMER.with(|slot| slot.replace(timer));
    }
}

extern "C" fn menu_did_close(_this: &mut Object, _cmd: Sel, _menu: id) {
    // Signal closed (if your refresh uses it).
    if let Some(cb) = REFRESH_CB.get() {
        cb(false);
    }

    // Stop and drop the timer immediately so clicking away is instant.
    TIMER.with(|slot| {
        let timer = *slot.borrow();
        if timer != nil {
            unsafe {
                let _: () = msg_send![timer, invalidate];
            }
            slot.replace(nil);
        }
    });
}

static MENU_DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl =
        ClassDecl::new("SysmonMenuDelegate", superclass).expect("Unable to declare delegate");

    // Add the delegate methods (unsafe per objc crate).
    unsafe {
        decl.add_method(sel!(menuWillOpen:), menu_will_open as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(sel!(menuDidClose:), menu_did_close as extern "C" fn(&mut Object, Sel, id));
    }

    decl.register()
});

/// Creates a delegate, attaches it to the menu, and returns it (so you can retain it).
pub fn attach_menu_delegate(_mt: &MainThreadToken, menu: &UiObj) -> UiObj {
    unsafe {
        let delegate: id = msg_send![*MENU_DELEGATE_CLASS, new];
        let _: () = msg_send![menu.as_id(), setDelegate: delegate];
        UiObj::from_raw_retained(delegate)
    }
}
