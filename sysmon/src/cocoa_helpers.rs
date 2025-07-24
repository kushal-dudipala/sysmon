use cocoa::appkit::NSMenuItem;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{msg_send, sel, sel_impl};

// These annotations confine the upstream objc macro’s noisy cfg to this file only.

#[allow(unexpected_cfgs)]
pub unsafe fn status_button(status_item: id) -> id {
    msg_send![status_item, button]
}

#[allow(unexpected_cfgs)]
pub unsafe fn set_button_title(button: id, s: &str) {
    let title = NSString::alloc(nil).init_str(s);
    let _: () = msg_send![button, setTitle: title];
}

#[allow(unexpected_cfgs)]
pub unsafe fn new_menu_item_with_title(s: &str) -> id {
    let _pool = NSAutoreleasePool::new(nil); // defensively pool ObjC allocs here too
    let item = NSMenuItem::new(nil).autorelease();
    let _: () = msg_send![item, setTitle: NSString::alloc(nil).init_str(s)];
    item
}

#[allow(unexpected_cfgs)]
pub unsafe fn menu_add_item(menu: id, item: id) {
    let _: () = msg_send![menu, addItem: item];
}

#[allow(unexpected_cfgs)]
pub unsafe fn status_item_set_menu(status_item: id, menu: id) {
    let _: () = msg_send![status_item, setMenu: menu];
}

#[allow(unexpected_cfgs)]
pub unsafe fn set_menu_item_title(item: id, s: &str) {
    let ns = NSString::alloc(nil).init_str(s);
    let _: () = msg_send![item, setTitle: ns];
}
