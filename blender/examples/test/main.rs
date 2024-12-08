use blender::models::home::BlenderHome;

fn test_download_blender_home_link() {
    let home = BlenderHome::new().expect("Unable to get data");
    let newest = home.as_ref().first().unwrap();
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
