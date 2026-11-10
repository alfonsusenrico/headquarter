//! Builds the Headquarter icon into the Windows executable (fix-windows-app D2). Explorer, the
//! taskbar and Alt+Tab read it, and GPUI loads it for the window as resource 1.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon/headquarter.ico");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icon/headquarter.ico");
        if let Err(error) = resource.compile() {
            panic!("could not build the Windows icon resource: {error}");
        }
    }
}
