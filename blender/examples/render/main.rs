use blender::blender::Blender;
use blender::blender::Manager;
use blender::models::status::Status;
use blender::models::{args::Args, mode::Mode};
use std::path::PathBuf;

fn render_with_manager() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // // we reference blender by executable path. Version will be detected upon running command process. (Self validation)
    let version = Blender::latest_version_available().unwrap();

    // if we have the manager available here...
    let mut manager = Manager::load();
    let blender = manager.get_blender(&version).unwrap();

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    let output = PathBuf::from("./examples/assets/");
    // Tells blender what kind of rendering mode are we performing, two options available, third one still in review for future impl.
    let mode = Mode::Frame(1);
    // let mode = Mode::Animation { start: 1, end: 2 };  // animation mode, requires two arguments, first frame to start render, and the last frame to end animation

    // Create blender argument, which is required for the argument to accept.
    let args = Args::new(blend_path, output, mode);

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    let listener = blender.render(args);
    // problem is, mpsc is not async. Need to wait for blender to finish rendering! :cry:
    while let Ok(status) = listener.recv() {
        match status {
            Status::Completed { result } => {
                println!("[Completed] {:?}", result);
                // break;
            }
            Status::Log { status } => {
                println!("[Info] {}", status);
            }
            Status::Running { status } => {
                println!("[Running] {}", status);
            }
            Status::Error(e) => {
                println!("[ERROR] {:?}", e);
                // break;
            }
            _ => {
                println!("unhandled blender status! {:?}", status);
            }
        }
    }
}

fn main() {
    render_with_manager();
}
