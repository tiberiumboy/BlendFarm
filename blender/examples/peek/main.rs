use std::{/*fs::OpenOptions, io::Write,*/ path::PathBuf};

// use blend::Blend;
use blender::manager::Manager as BlenderManager;

/// Peek into the blend file to see what's inside.
fn main() {
    /*
    let blend = Blend::from_path("./examples/assets/test.blend").expect("Invalid blender path provided");
    let mut file = OpenOptions::new().write(true).create(true).open("./log.txt").unwrap();

    for obj in blend.instances_with_code(b"SC") {
        // let loc = obj.get_f32_vec("loc");
        // let name = obj.get("id").get_string("name");
        // println!("\"{name}\" at {loc:?} | {obj:?}");
        let data = format!("{obj:?}\n");
        let _ = &file.write(data.as_bytes()).unwrap();
        // might be interesting to investigate the OBCamera for properties?
    }

    */
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };
    // // we reference blender by executable path. Version will be detected upon running command process. (Self validation)
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
