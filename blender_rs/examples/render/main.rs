use blender::blend_file::BlendFile;
use blender::blender::Manager;
use blender::models::engine::Engine;
use blender::models::{args::Args, event::BlenderEvent};
use semver::Version;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use xml_rpc::Value;

async fn render_with_manager() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    let blend_file = BlendFile::new(&blend_path).expect("Expects a valid blend file to continue!");

    // Get latest blender installed, or install latest blender from web.
    let mut manager = Manager::load();
    println!("Fetch latest available blender to use");

    let (max, min) = blend_file.get_partial_version();
    let version = Version::new(max as u64, min as u64, 0);

    let blender = manager
        .latest_local_avail(Some(&version))
        .expect("No local blender installation found! Must have at least one blender installed!");
    println!("Prepare blender configuration...");

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    // TODO: BUG! This will save to root of C:/ on windows platform! Need to change this to current working dir
    let output = PathBuf::from("./examples/assets/");

    // Create blender argument
    let args = Args::new(blend_file, output, Engine::BLENDER_EEVEE_NEXT);
    let frames = Arc::new(RwLock::new(RangeInclusive::new(2, 10)));

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    let listener = blender
        .render(
            args,
            Box::new(move |_params| {
                // need to convert this into XmlResponse
                match frames.write().unwrap().next() {
                    Some(frame) => Ok(Value::Int(frame).into()),
                    None => Err(Value::fault(-1, "No more frames to render!".to_owned())),
                }
            }),
        )
        .await
        .expect("Should not have any issue?");

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
