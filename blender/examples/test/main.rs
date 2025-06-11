use blender::manager::Manager;

fn test_download_blender_home_link() {
    let mut manager = Manager::load();
    let link = manager
        .latest_local_avail()
        .or(manager.download_latest_version().map_or(None, |l| Some(l)));
    match link {
        Some(link) => {
            dbg!(link);
        }
        None => println!("No blender found and unable to connect to internet! Skipping!"),
    }
}

fn main() {
    test_download_blender_home_link();
}
