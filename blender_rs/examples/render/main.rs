use blender::blender::Manager;
use blender::models::engine::Engine;
use blender::models::{args::Args, event::BlenderEvent};
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

async fn render_with_manager() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // Get latest blender installed, or install latest blender from web.
    let mut manager = Manager::load();
    println!("Fetch latest available blender to use");

    let blender = manager.latest_local_avail().unwrap_or_else(|| {
        println!("No local blender installation found! Downloading latest from internet...");
        manager
            .download_latest_version()
            .expect("Should be able to download blender! Are you not connected to the internet?")
    });

    println!("Prepare blender configuration...");

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    // TODO: BUG! This will save to root of C:/ on windows platform! Need to change this to current working dir
    let output = PathBuf::from("./examples/assets/");

    // Create blender argument
    let args = Args::new(blend_path, output, Engine::BLENDER_EEVEE_NEXT);
    let frames = Arc::new(RwLock::new(RangeInclusive::new(2, 10)));

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    let listener = blender
        .render(args, move || frames.write().unwrap().next())
        .await;

    // Handle blender status
    while let Ok(status) = listener.recv() {
        match status {
            BlenderEvent::Completed { frame, result } => {
                println!("[Completed] {frame} {result:?}");
            }
            BlenderEvent::Rendering { current, total } => {
                let percent = (current / total) * 100.0;
                println!("[Rendering] {current} out of {total} (%{percent})");
            }
            BlenderEvent::Error(e) => {
                println!("[ERR] {e}");
            }
            BlenderEvent::Warning(msg) => {
                println!("[WARN] {msg}");
            }
            BlenderEvent::Log(msg) => {
                println!("[LOG] {msg}")
            }
            BlenderEvent::Exit => {
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
