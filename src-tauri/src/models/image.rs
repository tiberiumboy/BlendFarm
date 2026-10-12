use std::path::{Path, PathBuf};

use blender_rs::blender::Frame;
use semver::Version;

use crate::models::job::JobId;

// an image struct will compose individual frame information.
pub struct Image {
    inner: PathBuf,        // path to image
    job_id: JobId,         // from which job this image was created for
    version: Version,      // Which blender version was this rendered from.
    project_file: PathBuf, // Which project file this image generated from.
    render_frame: Frame,
}

impl Image {
    pub fn new(
        path: impl AsRef<Path>,
        project_file: impl AsRef<Path>,
        job_id: JobId,
        version: Version,
        render_frame: Frame,
    ) -> Self {
        Self {
            inner: path.as_ref().to_path_buf(),
            job_id,
            version,
            project_file: project_file.as_ref().to_path_buf(),
            render_frame,
        }
    }
}
