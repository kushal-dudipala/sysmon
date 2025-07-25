use cocoa::appkit::NSMenuItem;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{msg_send, sel, sel_impl};

use crate::types::UiObj;

/// Returns the status button `id`.
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
