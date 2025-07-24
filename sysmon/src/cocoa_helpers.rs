use cocoa::appkit::NSMenuItem;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{msg_send, sel, sel_impl};
use crate::types::SendUiPtr;

pub fn status_button(status_item: id) -> id {
    unsafe { msg_send![status_item, button] }
}

pub fn set_button_title(ptr: SendUiPtr, s: &str) {
    unsafe {
        let id = ptr.0.as_ptr() as cocoa::base::id;
        let title = cocoa::foundation::NSString::alloc(nil).init_str(s);
        let _: () = objc::msg_send![id, setTitle: title];
    }
}

pub fn set_menu_item_title(ptr: SendUiPtr, s: &str) {
    unsafe {
        let id = ptr.0.as_ptr() as cocoa::base::id;
        let ns = cocoa::foundation::NSString::alloc(nil).init_str(s);
        let _: () = objc::msg_send![id, setTitle: ns];
    }
}

pub fn new_menu_item_with_title(s: &str) -> id {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let item = NSMenuItem::new(nil).autorelease();
        let title = NSString::alloc(nil).init_str(s);
        let _: () = msg_send![item, setTitle: title];
        item
    }
}

pub fn menu_add_item(menu: id, item: id) {
    unsafe {
        let _: () = msg_send![menu, addItem: item];
    }
}

pub fn status_item_set_menu(status_item: id, menu: id) {
    unsafe {
        let _: () = msg_send![status_item, setMenu: menu];
    }
}
