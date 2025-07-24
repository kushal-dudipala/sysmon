use std::ffi::c_void;
use std::ptr;
use std::os::raw::c_char;

use core_foundation::base::{CFTypeRef, CFRelease};
use libloading::{Library, Symbol};

type IOReportCopyChannelsInGroupFn = unsafe extern "C" fn(CFTypeRef, *const c_char, *const c_void) -> CFTypeRef;

pub fn get_temperature_channels() -> Option<CFTypeRef> {
    unsafe {
        let lib = Library::new("/System/Library/PrivateFrameworks/IOReport.framework/IOReport")
            .expect("Failed to open IOReport.framework");

        let func: Symbol<IOReportCopyChannelsInGroupFn> = lib.get(b"IOReportCopyChannelsInGroup\0")
            .expect("Failed to find IOReportCopyChannelsInGroup");

        let result = func(ptr::null_mut(), b"temperature\0".as_ptr() as *const c_char, ptr::null());

        if !result.is_null() {
            Some(result)
        } else {
            None
        }
    }
}
