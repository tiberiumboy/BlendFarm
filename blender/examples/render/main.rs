use std::path::PathBuf;
use BlendFarm::models::server_setting::ServerSetting;

fn main()
{
    let args = std::env::args().collect::<Vec<String>>();
    let blend_path = match args.get(1) {
        None => PathBuf::from("./test.blend"),
        Some(p) => PathBuf::from(p),
    };

    let server_settings = ServerSetting::load();
    let blender = server_settings.get_blenders().order_by(|x| x.version)
}