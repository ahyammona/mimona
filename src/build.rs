fn main() {
    // On Windows, don't show a console window when the app is launched
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    }
}