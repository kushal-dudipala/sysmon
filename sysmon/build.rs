use std::{env, fs, path::PathBuf};

fn main() {
    // Tell Cargo to rerun if the plist changes
    println!("cargo:rerun-if-changed=macos/Info.plist");

    // Copy plist to OUT_DIR so tools like cargo-bundle can pick it up
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::copy("macos/Info.plist", out_dir.join("Info.plist"))
        .expect("Failed to copy Info.plist");
    println!("cargo:warning=Copied Info.plist to OUT_DIR");
}
