//! Embed the exe icon: res/icon.rc compiles res/icon.ico into the binary so
//! Explorer, the taskbar and Alt+Tab show the same padlock as the tray.
//! rc.exe is located by embed-resource (PATH or the installed Windows SDK —
//! this box keeps it in the 10.0.26100 SDK, not on PATH). The release
//! workflow's windows-latest runner has the SDK as well.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=res/icon.rc");
    println!("cargo:rerun-if-changed=res/icon.ico");
    embed_resource::compile("res/icon.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("compile res/icon.rc (is the Windows SDK installed? set RC to rc.exe)");
}
