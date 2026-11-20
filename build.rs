fn main() {
    println!("cargo:rerun-if-changed=windows/application.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows/application.manifest");
        println!("cargo:rustc-link-arg-bin=Dado-Player=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=Dado-Player=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
