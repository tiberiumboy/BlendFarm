use super::with_id::WithId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub type CreatedRenderInfoDto = WithId<RenderInfo, Uuid>;
pub type NewRenderInfoDto = RenderInfo;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct RenderInfo {
    // what job this render image belongs to
    pub job_id: Uuid,

    // which frame
    pub frame: i32,

    // path to final image
    pub render_path: PathBuf,
}

impl RenderInfo {
    pub fn new(job_id: Uuid, frame: i32, path: impl AsRef<Path>) -> Self {
        Self {
            job_id,
            frame,
            render_path: path.as_ref().to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_new_succeed() {
        let job_id = Uuid::new_v4();
        let frame = 1i32;
        let path = Path::new("./test");

        let render_info = RenderInfo::new(job_id, frame, path);
        assert_eq!(render_info.job_id, job_id);
        assert_eq!(render_info.frame, frame);
        assert_eq!(render_info.render_path, path);
    }
}
