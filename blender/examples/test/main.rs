use blender::models::home::BlenderHome;

fn test_download_blender_home_link() {
    let mut home = BlenderHome::new().expect("Unable to get data");
    home.list.sort_by(|a, b| b.cmp(a));
    let newest = home.list.first().unwrap();
    let link = newest.fetch_latest();
    match link {
        Ok(link) => {
            dbg!(link);
        }
        Err(e) => println!("Something wrong - {e}"),
    }
}

fn main() {
    test_download_blender_home_link();
}
