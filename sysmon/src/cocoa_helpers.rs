use cocoa::appkit::{NSApplication, NSMenuItem};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use once_cell::sync::Lazy;

use crate::types::UiObj;

/* ---------------- Basic UI helpers  ---------------- */

/// SAFETY: Must be called from the main thread with a valid status item.
pub fn status_button(status_item: id) -> id {
    unsafe { msg_send![status_item, button] }
}

/// Safe wrapper to set a button's title.
pub fn set_button_title(ptr: &UiObj, title: &str) {
    unsafe {
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![ptr.as_id(), setTitle: title_str];
    }
}

/// Safe wrapper to set a menu item's title.
pub fn set_menu_item_title(ptr: &UiObj, title: &str) {
    unsafe {
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![ptr.as_id(), setTitle: title_str];
    }
}

/// Creates a new menu item wrapped in a `UiObj`.
/// SAFETY: Must be called on the main thread.
pub fn new_menu_item_with_title(title: &str) -> UiObj {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let item: id = NSMenuItem::new(nil).autorelease();
        assert!(!item.is_null(), "NSMenuItem::new returned null");
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![item, setTitle: title_str];
        UiObj::from_raw(item)
    }
}

/// Adds a menu item to a menu.
pub fn menu_add_item(menu: &UiObj, item: &UiObj) {
    unsafe {
        let _: () = msg_send![menu.as_id(), addItem: item.as_id()];
    }
}

/// Sets the menu for a status item.
pub fn status_item_set_menu(status_item: &UiObj, menu: &UiObj) {
    unsafe {
        let _: () = msg_send![status_item.as_id(), setMenu: menu.as_id()];
    }
}

/* ---------------- Quit menu  ---------------- */

extern "C" fn quit_gracefully(this: &Object, _cmd: Sel, _sender: id) {
    unsafe {
        // Invalidate timer first
        let timer: id = *this.get_ivar("timer");
        if !timer.is_null() {
            let _: () = msg_send![timer, invalidate];
        }

        // Terminate the app
        let app = NSApplication::sharedApplication(nil);
        let _: () = msg_send![app, terminate: this];
    }
}

static QUIT_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SysmonQuitTarget", superclass)
        .expect("Unable to declare SysmonQuitTarget class");

    // Store NSTimer* as an ivar
    decl.add_ivar::<id>("timer");

    decl.add_method(
        sel!(quitGracefully:),
        quit_gracefully as extern "C" fn(&Object, Sel, id),
    );
    decl.register()
});

fn make_quit_target(timer: id) -> id {
    unsafe {
        let target: id = msg_send![*QUIT_CLASS, new];
        (*target).set_ivar("timer", timer);
        target
    }
}

/// Create a “Quit sysmon” NSMenuItem that gracefully invalidates the timer first.
pub fn make_quit_menu_item(title: &str, timer: id) -> UiObj {
    unsafe {
        let key_equiv = NSString::alloc(nil).init_str(""); 
        let title_ns = NSString::alloc(nil).init_str(title);
        let item: id = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(title_ns, sel!(quitGracefully:), key_equiv)
            .autorelease();

        let target = make_quit_target(timer);
        let _: () = msg_send![item, setTarget: target];

        UiObj::from_raw(item)
    }
}
