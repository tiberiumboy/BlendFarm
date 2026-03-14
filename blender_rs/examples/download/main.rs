use ::blender::manager::Manager as BlenderManager;
use semver::Version;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let version = match args.get(1) {
        Some(v) => Version::parse(v).expect("Invalid version!"),
        None => return println!("Please, set a version number. E.g. 4.1.0"),
    };
    // We'll need a blender configuration file to use.
    let mut manager = BlenderManager::load(None).expect("Should have valid file?");
    let blender = manager
        .fetch_blender(&version)
        .expect("Unable to download Blender!");
    println!("Blender: {:?}", blender);
    assert_eq!(&version, blender.get_version());
}
