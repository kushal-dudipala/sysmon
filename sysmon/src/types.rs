use cocoa::base::id;
use objc::rc::StrongPtr;
use std::fmt;

/// Zero-sized token proving you're on the main thread.
/// Constructing it checks (and aborts if violated).
pub struct MainThreadToken(());

extern "C" {
    fn pthread_main_np() -> libc::c_int;
}

impl MainThreadToken {
    #[inline]
    pub fn acquire() -> Self {
        unsafe {
            if pthread_main_np() == 0 {
                eprintln!("UI code touched off the main thread!");
                std::process::abort();
            }
        }
        MainThreadToken(())
    }
}

/// Retained Objective‑C object with clear ownership semantics.
pub struct UiObj(StrongPtr);

impl UiObj {
    /// # Safety
    /// `obj` must be a valid Objective‑C object pointer.
    /// This retains it (+1). If `obj` was autoreleased, this intentionally
    /// over-retains and will live for the app lifetime (acceptable for this app).
    pub unsafe fn from_raw_retained(obj: id) -> Self {
        // explicit unsafe block required by `unsafe_op_in_unsafe_fn`
         let sp = unsafe { StrongPtr::retain(obj) };
        UiObj(sp)
    }

    #[inline]
    pub fn as_id(&self) -> id {
        *self.0
    }

    /// Explicit clone that retains (+1).
    pub fn clone_retained(&self) -> Self {
        UiObj(self.0.clone())
    }
}

impl fmt::Debug for UiObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UiObj({:p})", self.as_id())
    }
}
