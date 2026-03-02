use ::blender::manager::Manager as BlenderManager;
use ::blender::page_cache::PageCache;
use semver::Version;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let version = match args.get(1) {
        Some(v) => Version::parse(v).expect("Invalid version!"),
        None => return println!("Please, set a version number. E.g. 4.1.0"),
    };

    let mut page_cache = PageCache::load().expect("Should be able to load!");
    let mut manager = BlenderManager::load(&mut page_cache);
    let blender = manager
        .fetch_blender(&version)
        .expect("Unable to download Blender!");
    println!("Blender: {:?}", blender);
    assert_eq!(&version, blender.get_version());
}
