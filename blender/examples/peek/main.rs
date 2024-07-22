use std::path::PathBuf;

use blender::{blender::Blender, manager::Manager as BlenderManager};

/// Peek into the blend file to see what's inside.
fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // // we reference blender by executable path. Version will be detected upon running command process. (Self validation)
    let mut manager = BlenderManager::load();
    let blender = match manager.get_blenders().first() {
        None => {
            let version = Blender::latest_version_available().unwrap();
            manager.get_blender(&version).unwrap()
        }
        Some(blender) => blender.to_owned(),
    };

    match blender.peek(blend_path) {
        Ok(result) => println!("{:?}", &result),
        Err(e) => println!("Error: {:?}", e),
    }
}
