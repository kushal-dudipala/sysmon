use core_foundation::dictionary::*;
use core_foundation::string::*;
use core_foundation::base::*;
use core_foundation::array::*;
use std::ptr;

use crate::ioreport_bindings::*;

pub fn read_temp() {
    unsafe {
        let channels: CFArrayRef = IOReportCopyChannelsInGroup(CFSTR("temperature"), ptr::null_mut());
        if channels.is_null() {
            eprintln!("Failed to get temperature channels");
            return;
        }

        let array = CFArray::wrap_under_create_rule(channels);
        println!("Found {} temperature channels", array.len());

        for i in 0..array.len() {
            let item = array.get(i).unwrap();
            let name = IOReportChannelGetChannelName(item as _);
            let name_str = CFString::wrap_under_get_rule(name).to_string();
            println!("  - Channel {}: {}", i, name_str);
        }
    }
}
