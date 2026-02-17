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
use crate::domains::job_store::JobError;
use std::{ffi::OsStr, path::Path};
use blender::{blend_file::BlendFile, models::mode::RenderMode};
use futures::channel::mpsc::Sender;
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

#[derive(Debug)]
pub enum JobAction {
    Find(JobId, Sender<Option<CreatedJobDto>>),
    Update(CreatedJobDto),
    Create(NewJobDto, Sender<Result<CreatedJobDto, JobError>>),
    Kill(JobId),
    All(Sender<Option<Vec<CreatedJobDto>>>),
    // we will ask all of the node on the network if there's any completed job list.
    // The node will advertise their collection of completed job
    // the host will be responsible to compare with the current output files and 
    // see if there's any missing job. If there is missing frame then 
    // we will ask to fetch for that completed image back
    AskForCompletedList(JobId), 
    Advertise(JobId),
}

// Used to ignore sender types comparsion. We do not care about sender equality. 
impl PartialEq for JobAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Find(l0, ..), Self::Find(r0, ..)) => l0 == r0,
            (Self::Update(l0), Self::Update(r0)) => l0.id == r0.id,
            (Self::Create(l0, ..), Self::Create(r0,.. )) => l0 == r0,
            (Self::Kill(l0), Self::Kill(r0)) => l0 == r0,
            (Self::All(..), Self::All(..)) => true,
            (Self::AskForCompletedList(l0), Self::AskForCompletedList(r0)) => l0 == r0,
            (Self::Advertise(l0), Self::Advertise(r0)) => l0 == r0,
            _ => false,
        }
    }
}

pub type JobId = Uuid;
pub type Frame = i32;
pub type Output = PathBuf;
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
    blend_file: BlendFile,

    // target blender version
    blender_version: Version,

    // target output destination
    output: Output,
}

impl Job {
    // private - no validation, we trust that the validation is done from public api.
    fn new(
        mode: RenderMode,
        blend_file: BlendFile,
        blender_version: Version, // TODO: see if we can validate if this job uses the correct blender version
        output: Output,          // must be a valid directory
    ) -> Self {
        Self {
            mode,
            blend_file,
            blender_version,
            output,
        }
    }

    /// Create a new job entry with provided all information intact. Used for holding database records
    pub fn from(
        mode: RenderMode,
        project_file: &Path,
        version: Version,
        output: PathBuf,
    ) -> Result<Self, JobError> {
        match BlendFile::new(project_file) {
            Ok(file) => Ok(Job::new(mode, file, version, output)),
            Err(e) => Err(JobError::InvalidFile(e.to_string())),
        }
    }

    pub fn generate_task(self, id: Uuid) -> Option<Task> {
        // in this case, a job would have break up into pieces for worker client to receive and start a new job
        // first thing first, how can I tell if the job is completed or not?
        let range = self.clone().into();
        let job_id = WithId { id, item: self };
        
        match Task::from(job_id, range) {
            Ok(task) => Some(task),
            Err(e) => {
                println!("Unable to make task? {e:?}");
                None
            }
        }
    }

    pub fn get_file_name_expected(&self) -> &OsStr {
        self.blend_file.to_path().file_name().expect("Must have valid file name already")
    }
}

impl AsRef<BlendFile> for Job {
    fn as_ref(&self) -> &BlendFile {
        &self.blend_file
    }
}

impl AsRef<Version> for Job {
    fn as_ref(&self) -> &Version {
        &self.blender_version
    }
}

/// return the job output destination (Should be used on the host machine)
impl AsRef<Output> for Job {
    fn as_ref(&self) -> &Output {
        &self.output
    }
}

impl AsRef<RenderMode> for Job {
    fn as_ref(&self) -> &RenderMode {
        &self.mode
    }
}

// TODO: Clone/to_owned() is used here.
impl Into<Range<i32>> for Job {
    fn into(self) -> Range<i32> {
        match self.mode {
            RenderMode::Animation(range) => range.clone(),
            RenderMode::Frame(frame) => Range {
                start: frame.to_owned(),
                end: frame.to_owned(),
            },
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::models::constant::test::{EXAMPLE_FILE, EXAMPLE_OUTPUT};
    use std::path::Path;

    pub fn scaffold_job() -> Job {
        let mode = RenderMode::Frame(1);
        let file = Path::new(EXAMPLE_FILE);
        let project_file =
            BlendFile::new(file).expect("expect this to work without issue");
        let version = Version::new(4, 4, 0);
        let dir = Path::new(EXAMPLE_OUTPUT);
        let output = dir.to_path_buf();
        Job::new(mode, project_file, version, output)
    }

    // we should at least try to test it against public api
    #[test]
    fn create_job_successful() {
        let file = Path::new(EXAMPLE_FILE);
        let mode = RenderMode::Frame(1);
        let version = Version::new(1, 1, 1);
        let output = Path::new("./test/");
        let job = Job::from(
            mode.clone(),
            file,
            version.clone(),
            output.to_path_buf(),
        );

        let project_file =
            BlendFile::new(file).expect("Should be valid project file");

        assert!(job.is_ok());
        let job = job.unwrap();

        assert_eq!(job.mode, mode);
        assert_eq!(job.output, output);
        assert_eq!(AsRef::<BlendFile>::as_ref(&job), &project_file);
        assert_eq!(AsRef::<Version>::as_ref(&job), &version);
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
