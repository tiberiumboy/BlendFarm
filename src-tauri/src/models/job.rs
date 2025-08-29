/*
    Developer Blog:
    - Original idea behind this was to use PhantomData to mitigate the status of the job instead of reading from enum.
        Need to refresh materials about PhantomData, and how I can translate this data information for front end to update/reflect changes
        The idea is to change the struct to have state of the job.
        I think the limitation for this is serialization/deserialization property.
    - I need to fetch the handles so that I can maintain and monitor all node activity.
    - TODO: See about migrating Sender code into this module?
*/
use super::task::Task;
use super::with_id::WithId;
use crate::{domains::job_store::JobError, models::project_file::ProjectFile};
use blender::models::mode::RenderMode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{ops::Range, path::PathBuf};
use uuid::Uuid;
use crate::network::PeerIdString;

#[derive(Debug, Serialize, Deserialize)]
pub enum JobEvent {
    Render(PeerIdString, Task),
    Remove(Uuid),
    Failed(String),
    RequestTask(PeerIdString),
    ImageCompleted {
        job_id: Uuid,
        frame: Frame,
        file_name: String,
    },
    AskForCompletedJobFrameList(JobId),
    ImageCompletedList {
        job_id: JobId,
        files: Vec<String>,
    },
    TaskComplete, // what's the difference between JobComplete and TaskComplete?
    Error(JobError),
}

pub type JobId = Uuid;
pub type Frame = i32;
pub type NewJobDto = Job;
pub type CreatedJobDto = WithId<Job, JobId>;

// This job is created by the manager and will be used to help determine the individual task created for the workers
// we will derive this job into separate task for individual workers to process based on chunk size.
#[derive(
    Debug, Serialize, Deserialize, Clone, sqlx::FromRow, sqlx::Encode, sqlx::Decode, PartialEq,
)]
pub struct Job {
    /// contains the information to specify the kind of job to render (We could auto fill this from blender peek function?)
    mode: RenderMode,

    /// Path to blender files
    project_file: ProjectFile,

    // target blender version
    blender_version: Version,

    // target output destination
    output: PathBuf, // is there a way to say that this is exactly the directory path instead of pathbuf?
}

impl Job {
    // private - no validation, we trust that the validation is done via public api.
    fn new(
        mode: RenderMode,
        project_file: ProjectFile,
        blender_version: Version, // TODO: see if we can validate if this job uses the correct blender version
        output: PathBuf,          // must be a valid directory
    ) -> Self {
        Self {
            mode,
            project_file,
            blender_version,
            output,
        }
    }

    /// Create a new job entry with provided all information intact. Used for holding database records
    pub fn from(
        mode: RenderMode,
        project_file: PathBuf,
        version: Version,
        output: PathBuf,
    ) -> Result<Self, JobError> {
        match ProjectFile::from(project_file) {
            Ok(file) => Ok(Job::new(mode, file, version, output)),
            Err(e) => Err(JobError::InvalidFile(e.to_string())),
        }
    }

    pub fn generate_task(self, id: Uuid) -> Option<Task> {
        // in this case, a job would have break up into pieces for worker client to receive and start a new job
        // first thing first, how can I tell if the job is completed or not?
        let range = self.get_range();
        let job = WithId { id, item: self };
        match Task::from(job, range) {
            Ok(task) => Some(task),
            Err(e) => {
                println!("Unable to make task? {e:?}");
                None
            }
        }
    }

    pub fn get_range(&self) -> Range<i32> {
        match self.get_mode() {
            RenderMode::Animation(range) => range.clone(),
            RenderMode::Frame(frame) => Range {
                start: frame.to_owned(),
                end: frame.to_owned(),
            },
        }
    }

    pub fn get_mode(&self) -> &RenderMode {
        &self.mode
    }

    // TODO: See if there's a better way to obtain file name, project path, and version
    pub fn get_file_name_expected(&self) -> &str {
        // this line could potentially break the application
        // if the project file was malform or set to use directory instead.
        self.project_file.file_name().unwrap().to_str().unwrap()
    }

    pub fn get_project_path(&self) -> &ProjectFile {
        &self.project_file
    }

    pub fn get_version(&self) -> &Version {
        &self.blender_version
    }

    /// return the job output destination (Should be used on the host machine)
    pub fn get_output(&self) -> &PathBuf {
        &self.output
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use std::path::Path;

    pub fn scaffold_job() -> Job {
        let mode = RenderMode::Frame(1);
        // getting build failure that I cannot open blend file
        // TODO: how do I load path from project directory>
        let project_file = Path::new("./blender_rs/examples/assets/test.blend").to_path_buf();
        let project_file =
            ProjectFile::from(project_file).expect("expect this to work without issue");
        let version = Version::new(4, 4, 0);
        let output = Path::new("./blender_rs/examples/assets/").to_path_buf();
        Job::new(mode, project_file, version, output)
    }

    // we should at least try to test it against public api
    #[test]
    fn create_job_successful() {
        let mode = RenderMode::Frame(1);
        let file = Path::new("./test.blend");
        let version = Version::new(1, 1, 1);
        let output = Path::new("./test/");
        let job = Job::from(
            mode.clone(),
            file.to_path_buf(),
            version.clone(),
            output.to_path_buf(),
        );

        let project_file =
            ProjectFile::from(file.to_path_buf()).expect("Should be valid project file");

        assert!(job.is_ok());
        let job = job.unwrap();

        assert_eq!(job.mode, mode);
        assert_eq!(job.output, output);
        assert_eq!(job.get_project_path(), &project_file);
        assert_eq!(job.get_version(), &version);
        assert_eq!(
            job.get_file_name_expected(),
            file.file_name()
                .expect("Should have valid file name")
                .to_str()
                .expect("Shoudl have valid file name!")
        );
    }

    #[test]
    fn invalid_project_file_path_should_fail() {}
}
