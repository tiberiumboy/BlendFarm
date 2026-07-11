use blender::blend_file::BlendFile;
use blender::blender::get_blend_config_from_local;
use blender::blender::Manager;
use blender::models::event::RenderEvent;
use blender::models::{args::Args, event::BlenderEvent};
use semver::Version;
use std::fs;
use std::path::PathBuf;

async fn render_with_manager() {
    let args = std::env::args().collect::<Vec<String>>();

    let blend_path = match args.get(1) {
        // FIXME: Path is relative to where command is invoked. Must be from blender_rs directory, otherwise path will fail.
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // loads blender file and retrieve some information to display for job queue.
    let blend_file = BlendFile::new(&blend_path).expect("Expects a valid blend file to continue!");

    let config = get_blend_config_from_local().expect("Unable to get blend config!");

    // Get latest blender installed, or install latest blender from web.
    let manager = Manager::load(config).expect("Must be able to launch manager to get blender");

    // Retrieve last blender version opened/used. Only contains major and minor, no patch. Rely on latest patch if possible.
    let (max, min) = blend_file.get_partial_version();

    // Minimum version required to run this blender file
    let version = Version::new(max as u64, min as u64, 0);

    // Fetch latest local version that meets the requirement version. We will not try to install,
    // so we will stop here and ask the user to load blender into configuration initially.
    let config = manager.get_config();
    let blender = config
        .get_latest_blender_available(&version)
        .expect("No local blender installation found! Must have at least one blender installed!");
    println!("Prepare blender configuration...");

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    // TODO: BUG! This will save to root of C:/ on windows platform! Need to change this to current working dir
    // Do not have a copy of windows to resolve this path convention.
    let output = fs::canonicalize(PathBuf::from("./examples/assets/"))
        .expect("Must be able to collapse to absolute path!");

    // Create blender argument
    let args = Args::new(blend_file, output, 2, 3);

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    let listener = blender
        .render(args)
        .await
        .expect("Should not have any issue?");

    // Handle blender status
    while let Ok(status) = listener.recv() {
        match status {
            BlenderEvent::Rendering(render_event) => match render_event {
                RenderEvent::Progress {
                    frame,
                    current,
                    total,
                } => {
                    let percent = (current / total) * 100.0;
                    println!("[Rendering] frame: {frame} | {current} out of {total} (%{percent})");
                }
                RenderEvent::Complete { frame, path } => {
                    println!("[Completed] frame: {frame} | path: {path:?}");
                }
            },
            BlenderEvent::Error(e) => {
                println!("[ERR] {e}");
            }
            BlenderEvent::Warning(msg) => {
                println!("[WARN] {msg}");
            }
            BlenderEvent::Info(msg) => {
                println!("[LOG] {msg}")
            }
            BlenderEvent::Exit => {
                println!("[Exit]");
                break;
            }
            _ => {
                println!("Unhandled blender status! {:?}", status);
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    render_with_manager().await;
}
