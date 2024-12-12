use blender::blender::Manager;
use blender::models::{args::Args, mode::Mode, status::Status};
use std::path::PathBuf;

async fn render_with_manager() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // Get latest blender installed, or install latest blender from web.
    let mut manager = Manager::load();
    let blender = match manager.latest_local_avail() {
        Some(blender) => blender,
        None => manager
            .download_latest_version()
            .expect("Should be able to download blender! Are you not connected to the internet?"),
    };

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    // TODO: BUG! This will save to root of C:/ on windows platform! Need to change this to current working dir
    let output = PathBuf::from("./examples/assets/");

    // Tells blender what kind of rendering mode are we performing, two options available, third one still in review for future impl.
    // let mode = Mode::Frame(1);
    let mode = Mode::Animation { start: 2, end: 6 };

    // Create blender argument
    let args = Args::new(blend_path, output, mode);

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    let listener = blender.render(args).await;

    // Handle blender status
    while let Ok(status) = listener.recv() {
        match status {
            Status::Completed { frame, result } => {
                println!("[Completed] {frame} {result:?}");
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
            Status::Exit => {
                println!("[Exit]");
            }
            _ => {
                println!("Unhandled blender status! {:?}", status);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    render_with_manager().await;
}
