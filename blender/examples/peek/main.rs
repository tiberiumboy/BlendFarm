use blender::blender::Blender;
use std::path::PathBuf;

/// Peek into the blend file to see what's inside.
#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./examples/assets/test.blend"),
        Some(p) => PathBuf::from(p),
    };

    // we reference blender by executable path. Version will be detected upon running command process. (Self validation)
    match Blender::peek(&blend_path).await {
        Ok(result) => println!("{:?}", &result),
        Err(e) => println!("Error: {:?}", e),
    }
}
