fn main() {
    println!("cargo:rerun-if-env-changed=Y2D_INSTALLER_PAYLOAD");
    if let Ok(payload) = std::env::var("Y2D_INSTALLER_PAYLOAD") {
        println!("cargo:rerun-if-changed={payload}");
    }
    tauri_build::build()
}
