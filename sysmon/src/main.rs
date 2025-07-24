mod ioreport;

fn main() {
    unsafe {
        if let Some(channels) = ioreport::get_temperature_channels() {
            println!("Successfully retrieved temperature channels!");
            // Later you can decode channels here
            // For now just release it
            core_foundation::base::CFRelease(channels);
        } else {
            eprintln!("Failed to get temperature channels.");
        }
    }
}
