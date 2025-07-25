use cocoa::appkit::NSMenuItem;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use dispatch::Queue;
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use once_cell::sync::{Lazy, OnceCell};
use std::ffi::c_void;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use crate::types::{MainThreadToken, UiObj};

/* ---------------- Refresh callback plumbing ---------------- */

/// `opened = true` while menu is open, false when it closes.
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


/* ---------------- Quit Callback ---------------- */

extern "C" fn quit_immediately(_this: &Object, _cmd: Sel, _sender: id) {
    std::process::exit(0);
}

static QUIT_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("SysmonQuitTarget", superclass)
        .expect("Unable to declare SysmonQuitTarget class");

    decl.add_method(
        sel!(quitImmediately:),
        quit_immediately as extern "C" fn(&Object, Sel, id),
    );

    decl.register()
});

pub fn make_quit_menu_item(_mt: &MainThreadToken, title: &str) -> (UiObj, UiObj) {
    unsafe {
        let title_ns = NSString::alloc(nil).init_str(title);
        let key_equiv = NSString::alloc(nil).init_str("");

        let item: id = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(title_ns, sel!(quitImmediately:), key_equiv)
            .autorelease();

        let target = msg_send![*QUIT_CLASS, new];
        let _: () = msg_send![item, setTarget: target];

        (
            UiObj::from_raw_retained(item),
            UiObj::from_raw_retained(target), 
        )
    }
}

/* ---------------- NSMenuDelegate using a background Rust thread ---------------- */

/// Boxed thread-control handle we store in an ivar as a raw pointer.
struct ThreadTimer {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}
type ThreadPtr = *mut c_void;

extern "C" fn menu_will_open(this: &mut Object, _cmd: Sel, _menu: id) {
    // Tell app we opened (do an immediate refresh)
    if let Some(cb) = REFRESH_CB.get() {
        cb(true);
    }

    unsafe {
        // Stop any previous timer thread
        let existing: ThreadPtr = *this.get_ivar("thread_ptr");
        if !existing.is_null() {
            let mut boxed: Box<ThreadTimer> = Box::from_raw(existing as *mut ThreadTimer);
            boxed.stop.store(true, Ordering::Relaxed);
            if let Some(h) = boxed.handle.take() {
                let _ = h.join();
            }
        }

        // Spawn a new background thread that pings main every 1s
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::spawn(move || {
            let q = Queue::main();
            while !stop_clone.load(Ordering::Relaxed) {
                // hop back to main to do the UI refresh
                if let Some(cb) = REFRESH_CB.get() {
                    q.exec_async(move || cb(true));
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        let tt = ThreadTimer {
            stop,
            handle: Some(handle),
        };
        let raw: ThreadPtr = Box::into_raw(Box::new(tt)) as ThreadPtr;
        this.set_ivar("thread_ptr", raw);
    }
}

extern "C" fn menu_did_close(this: &mut Object, _cmd: Sel, _menu: id) {
    // Optionally notify (we don't use opened=false in refresh right now, but it's here)
    if let Some(cb) = REFRESH_CB.get() {
        cb(false);
    }

    unsafe {
        let raw: ThreadPtr = *this.get_ivar("thread_ptr");
        if !raw.is_null() {
            let mut boxed: Box<ThreadTimer> = Box::from_raw(raw as *mut ThreadTimer);
            boxed.stop.store(true, Ordering::Relaxed);
            if let Some(h) = boxed.handle.take() {
                let _ = h.join();
            }
            this.set_ivar::<ThreadPtr>("thread_ptr", std::ptr::null_mut::<c_void>() as ThreadPtr);
        }
    }
}

static MENU_DELEGATE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl =
        ClassDecl::new("SysmonMenuDelegate", superclass).expect("Unable to declare delegate");

    // dispatch-thread controller as void*
    decl.add_ivar::<ThreadPtr>("thread_ptr");

    decl.add_method(
        sel!(menuWillOpen:),
        menu_will_open as extern "C" fn(&mut Object, Sel, id),
    );
    decl.add_method(
        sel!(menuDidClose:),
        menu_did_close as extern "C" fn(&mut Object, Sel, id),
    );

    decl.register()
});

/// Creates a delegate, attaches it to the menu, and returns it (so you can retain it).
pub fn attach_menu_delegate(_mt: &MainThreadToken, menu: &UiObj) -> UiObj {
    unsafe {
        let delegate: id = msg_send![*MENU_DELEGATE_CLASS, new];
        let delegate_obj: &mut Object = &mut *(delegate as *mut Object);
        delegate_obj.set_ivar::<ThreadPtr>("thread_ptr", std::ptr::null_mut::<c_void>() as ThreadPtr);


        let _: () = msg_send![menu.as_id(), setDelegate: delegate];

        UiObj::from_raw_retained(delegate)
    }
}
