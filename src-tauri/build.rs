fn main() {
    tauri_build::build();
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");
}
