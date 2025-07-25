use cocoa::appkit::{NSApplication, NSMenuItem};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use once_cell::sync::{Lazy, OnceCell};

use crate::types::{MainThreadToken, UiObj};

/* ---------------- Refresh callback plumbing ---------------- */

type RefreshFn = fn();

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

/* ---------------- Quit menu ---------------- */

extern "C" fn quit_gracefully(this: &Object, _cmd: Sel, _sender: id) {
    unsafe {
        // Terminate the app
        let app = NSApplication::sharedApplication(nil);
        let _: () = msg_send![app, terminate: this];
    }
}

static QUIT_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SysmonQuitTarget", superclass)
        .expect("Unable to declare SysmonQuitTarget class");

    decl.add_method(
        sel!(quitGracefully:),
        quit_gracefully as extern "C" fn(&Object, Sel, id),
    );
    decl.register()
});

fn make_quit_target() -> id {
    unsafe { msg_send![*QUIT_CLASS, new] }
}

pub fn make_quit_menu_item(_mt: &MainThreadToken, title: &str) -> UiObj {
    unsafe {
        let key_equiv = NSString::alloc(nil).init_str("");
        let title_ns = NSString::alloc(nil).init_str(title);
        let item: id = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(title_ns, sel!(quitGracefully:), key_equiv)
            .autorelease();

        let target = make_quit_target();
        let _: () = msg_send![item, setTarget: target];

        UiObj::from_raw_retained(item)
    }
}

/* ---------------- NSMenuDelegate to refresh on click ---------------- */

extern "C" fn menu_will_open(_this: &Object, _cmd: Sel, _menu: id) {
    if let Some(cb) = REFRESH_CB.get() {
        cb();
    }
}

static MENU_DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl =
        ClassDecl::new("SysmonMenuDelegate", superclass).expect("Unable to declare delegate");

    decl.add_method(
        sel!(menuWillOpen:),
        menu_will_open as extern "C" fn(&Object, Sel, id),
    );

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
