use cocoa::base::id;
use objc::rc::StrongPtr;

#[derive(Clone)] // Drop Debug — StrongPtr doesn't implement it
pub struct UiObj(StrongPtr);

impl UiObj {
    /// # Safety
    /// `obj` must be a valid Objective‑C object pointer.
    pub unsafe fn from_raw(obj: id) -> Self {
        // Because of `#![deny(unsafe_op_in_unsafe_fn)]`, wrap the retain in its own `unsafe` block.
        let sp = unsafe { StrongPtr::retain(obj) };
        UiObj(sp)
    }

    #[inline]
    pub fn as_id(&self) -> id {
        *self.0
    }
}

 // debug
impl std::fmt::Debug for UiObj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UiObj({:p})", self.as_id())
    }
}
