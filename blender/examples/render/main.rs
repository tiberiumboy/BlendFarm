use blender::blender::Manager;
use blender::models::{args::Args, mode::Mode, status::Status};
use std::path::PathBuf;

fn render_with_manager() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // Get latest blender installed, or install latest blender from web.
    let mut manager = Manager::load();
    let blender = match manager.latest_local_avail() {
        Some(blender) => blender,
        None => manager.download_latest_version().unwrap(),
    };

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    let output = PathBuf::from("./examples/assets/");

    // Tells blender what kind of rendering mode are we performing, two options available, third one still in review for future impl.
    let mode = Mode::Frame(1);
    // let mode = Mode::Animation { start: 1, end: 2 };

    // Create blender argument
    let args = Args::new(blend_path, output, mode);

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    let listener = blender.render(args);

    // Handle blender status
    while let Ok(status) = listener.recv() {
        match status {
            Status::Completed { result } => {
                println!("[Completed] {:?}", result);
            }
            Status::Log { status } => {
                println!("[Info] {}", status);
            }
            Status::Running { status } => {
                println!("[Running] {}", status);
            }
            Status::Error(e) => {
                println!("[ERROR] {:?}", e);
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
