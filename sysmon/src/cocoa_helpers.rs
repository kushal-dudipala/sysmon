use cocoa::appkit::NSMenuItem;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{msg_send, sel, sel_impl};
use objc::runtime::Object;

use crate::types::SendUiPtr;

/// SAFETY: Must be called from the main thread with a valid status item.
pub fn status_button(status_item: id) -> *mut Object {
    unsafe { msg_send![status_item, button] }
}

/// Safe wrapper to set a button's title.
pub fn set_button_title(ptr: &SendUiPtr, title: &str) {
    unsafe {
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![ptr.as_ptr(), setTitle: title_str];
    }
}

/// Safe wrapper to set a menu item's title.
pub fn set_menu_item_title(ptr: &SendUiPtr, title: &str) {
    unsafe {
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![ptr.as_ptr(), setTitle: title_str];
    }
}

/// Creates a new menu item wrapped in a `SendUiPtr`.
/// SAFETY: Must be called on the main thread.
pub fn new_menu_item_with_title(title: &str) -> SendUiPtr {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let item: id = NSMenuItem::new(nil).autorelease();
        let title_str = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![item, setTitle: title_str];
        SendUiPtr::new(item).expect("created menu item was null")
    }
}

/// Adds a menu item to a menu.
/// SAFETY: Both `menu` and `item` must be valid Objective-C objects.
pub fn menu_add_item(menu: &SendUiPtr, item: &SendUiPtr) {
    unsafe {
        let _: () = msg_send![menu.as_ptr(), addItem: item.as_ptr()];
    }
}

/// Sets the menu for a status item.
/// SAFETY: Must be called from main thread with valid pointers.
pub fn status_item_set_menu(status_item: &SendUiPtr, menu: &SendUiPtr) {
    unsafe {
        let _: () = msg_send![status_item.as_ptr(), setMenu: menu.as_ptr()];
    }
}
