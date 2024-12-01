use blender::manager::Manager as BlenderManager;
use std::path::PathBuf;

/// Peek into the blend file to see what's inside.
fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // we reference blender by executable path. Version will be detected upon running command process. (Self validation)
    let mut manager = BlenderManager::load();
    let blender = match manager.get_blenders().first() {
        Some(blender) => blender.to_owned(),
        None => manager.download_latest_version().unwrap(),
    };

    match blender.peek(blend_path) {
        Ok(result) => println!("{:?}", &result),
        Err(e) => println!("Error: {:?}", e),
    }
}
