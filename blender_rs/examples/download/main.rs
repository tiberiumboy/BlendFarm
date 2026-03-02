use ::blender::manager::Manager as BlenderManager;
use blender::page_cache;
use semver::Version;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let version = match args.get(1) {
        Some(v) => Version::parse(v).expect("Invalid version!"),
        None => return println!("Please, set a version number. E.g. 4.1.0"),
    };

    let page_cache = PageCache::load();
    let mut manager = BlenderManager::load(page_cache);
    let blender = manager
        .fetch_blender(&version)
        .expect("Unable to download Blender!");
    println!("Blender: {:?}", blender);
    assert_eq!(&version, blender.get_version());
}
