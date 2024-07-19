use std::{fs, path::PathBuf};

use ::blender::blender::Blender;
use semver::Version;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let version = match args.get(1) {
        Some(v) => Version::parse(v).expect("Invalid version!"),
        None => return println!("Please, set a version number. E.g. 4.1.0"),
    };

    let install_path = match args.get(2) {
        Some(p) => PathBuf::from(p),
        // by default, if the user doesn't supply download path location, we will use the current user's download location instead.
        None => {
            let download_dir =
                dirs::download_dir().expect("Unable to get default blender download location!");
            let download_dir = download_dir.join("blender");
            let _result = fs::create_dir_all(&download_dir);
            download_dir
        }
    };

    let blender = Blender::download(version, install_path).expect("Unable to download Blender!");
    println!("Blender downloaded at: {:?}", blender.get_executable());
}
