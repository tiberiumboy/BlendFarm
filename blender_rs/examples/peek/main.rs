use blender::blend_file::BlendFile;
use std::path::PathBuf;

/// Peek into the blend file to see what's inside.
#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        // Note this would only work if you ran the example from /blender_rs directory
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    match BlendFile::new(&blend_path) {
        Ok(result) => println!("{:?}", &result.peek_response(None)),
        Err(e) => println!("Error: {:?}", e),
    }
}
