use semvar::Version;
use Uuid::Uuid;

struct RenderTask {
    pub id: Uuid,
    pub session_id: i32,
    pub version: Version,
    pub file: File,
    pub settings: RenderSettings,
}

impl RenderTask {
    fn new() -> RenderTask {}
}
