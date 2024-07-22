use std::path::PathBuf;

use blender::blender::Blender;

/// Peek into the blend file to see what's inside.
fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // fetch a blender installation somehow?
    let blender_path = dirs::download_dir().expect("Unable to get your download path!");
    // // because I'm hardcoding this path, I wanted to make sure this all still continue to work before I complete this example exercise.
    // // This is a hack I want to get around. If we can't get blender installation, then we need to politely ask user to create one.
    let blender_path = blender_path.join("blender");

    // // we reference blender by executable path. Version will be detected upon running command process. (Self validation)
    let version = Blender::latest_version_available().unwrap();
    let blender = Blender::download(version, blender_path).unwrap();

    match blender.peek(blend_path) {
        Ok(result) => println!("{:?}", &result),
        Err(e) => println!("Error: {:?}", e),
    }
}
