use objc::runtime::Object;
use std::ptr::NonNull;

#[derive(Debug, PartialEq, Clone)]
pub struct SendUiPtr(pub NonNull<Object>);

impl SendUiPtr {
    pub fn new(ptr: *mut Object) -> Option<Self> {
        NonNull::new(ptr).map(Self)
    }

    pub fn as_ptr(&self) -> *mut Object {
        self.0.as_ptr()
    }
}


// SAFETY: Pointer assumed valid and pinned to main thread.
// unsafe impl Send for SendUiPtr {}
