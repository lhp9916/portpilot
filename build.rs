#[cfg(windows)]
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource
            .set_icon("assets/icon.ico")
            .set("FileDescription", "PortPilot local port forwarding")
            .set("ProductName", "PortPilot")
            .set("ProductVersion", env!("CARGO_PKG_VERSION"))
            .set("FileVersion", env!("CARGO_PKG_VERSION"));
        if let Err(error) = resource.compile() {
            panic!("failed to compile Windows resources: {error}");
        }
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            let resource_object =
                PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set"))
                    .join("resource.o");
            println!("cargo:rustc-link-arg={}", resource_object.display());
        }
    }
}
