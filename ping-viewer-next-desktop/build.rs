fn main() {
    // Embed the icon into the Windows executable (Explorer, taskbar, shortcuts).
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=icons/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("icons/icon.ico")
            .compile()
            .expect("failed to embed Windows icon");
    }
}
