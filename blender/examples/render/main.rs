use std::path::PathBuf;

use blender::{args::Args, blender::Blender, mode::Mode};

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/render/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // fetch a blender installation somehow?
    // let server_settings = ServerSetting::load();
    // let blender =server_settings.get_blenders().order_by(|x| x.version);
    let blender_path = dirs::download_dir().expect("Unable to get your download path!");
    // because I'm hardcoding this path, I wanted to make sure this all still continue to work before I complete this example exercise.
    let blender_path = blender_path
        .join("blender")
        .join("Blender4.1")
        .join("blender-4.1.0-macos-arm64")
        .join("Blender.app");

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    let output = PathBuf::from("./examples/render/");
    // Tells blender what kind of rendering mode are we performing, two options available, third one still in review for future impl.
    let mode = Mode::Frame(1);
    // let mode = Mode::Animation { start: 1, end: 2 };  // animation mode, requires two arguments, first frame to start render, and the last frame to end animation

    // Create blender argument, which is required for the argument to accept.
    let args = Args::new(blend_path, output, mode);

    // we reference blender by executable path. Version will be detected upon running command process. (Self validation)
    let blender = match Blender::from_executable(blender_path) {
        Ok(blender) => blender,
        Err(e) => {
            panic!("unable to get blender from executable path! \n{e}");
        }
    };

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    match blender.render(&args) {
        Ok(path) => println!("Render completed! {}", path),
        Err(e) => println!("Fail to render! \n{e}"),
    }
}
