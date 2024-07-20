use blender::blender::Blender;
use blender::models::status::Status;
use blender::models::{args::Args, mode::Mode};
use std::path::PathBuf;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/render/test.blend"),
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

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    let output = PathBuf::from("./examples/render/");
    // Tells blender what kind of rendering mode are we performing, two options available, third one still in review for future impl.
    let mode = Mode::Frame(1);
    // let mode = Mode::Animation { start: 1, end: 2 };  // animation mode, requires two arguments, first frame to start render, and the last frame to end animation

    // Create blender argument, which is required for the argument to accept.
    let args = Args::new(blend_path, output, mode);

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    blender.render(&args);
    // problem is, mpsc is not async. Need to wait for blender to finish rendering! :cry:
    while let Ok(status) = blender.listener.recv() {
        match status {
            Status::Completed { result } => {
                println!("[Completed] {:?}", result);
                blender.stop();
                break;
            }
            Status::Running { status } => {
                println!("[Running] {}", status);
            }
            Status::Error { message } => {
                println!("[ERROR] {:?}", message);
                blender.stop();
                break;
            }
            _ => {
                println!("unhandled blender status! {:?}", status);
            }
        }
    }
}
