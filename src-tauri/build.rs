fn main() {
    // Rebuild Windows resources when icons change, including resource-only updates.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
