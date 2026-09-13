use std::{path::PathBuf, str::FromStr};

use crate::{
    domains::job_store::{JobError, JobStore},
    models::{
        job::{CreatedJobDto, Job, NewJobDto, Output},
        with_id::WithId,
    },
};
use blender_rs::blend_file::BlendFile;
use blender_rs::models::mode::RenderMode;
use semver::Version;
use sqlx::{FromRow, SqlitePool, query_as, types::Json};
use uuid::Uuid;

pub struct SqliteJobStore {
    conn: SqlitePool,
}

impl SqliteJobStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

// this information is used to help transpose data into database format.
#[derive(Debug, Clone, FromRow)]
struct JobDAO {
    id: String,
    mode: Json<RenderMode>,
    project_file: String,
    // This is Version (major.minor.patch)
    blender_version: Json<Version>,
    output_path: String,
}

impl JobDAO {
    pub fn dto_to_obj(self) -> Result<WithId<Job, Uuid>, JobError> {
        // I had issue with converting job record into job struct.
        // We should not trust where the project file until we actually execute and invoke the job.
        // for now, we will trust what the record has provided us as scaffolding starting point where the file should be located.
        // This would be a great idea to make use of PhantomData, mark certain struct implementation to verify we have the actual job to run from.
        let project_file = PathBuf::from_str(&self.project_file)
            .map_err(|e| JobError::InvalidFile(e.to_string()))?;

        let id = Uuid::from_str(&self.id).map_err(|e| JobError::DatabaseError(e.to_string()))?;
        // let mode =
        //     serde_json::from_str(&self.mode).map_err(|e| JobError::DatabaseError(e.to_string()))?;
        let mode = self.mode.into_inner();

        // let blender_version = Version::from_str(&self.blender_version)
        //     .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        let blender_version = self.blender_version.into_inner();

        let output = PathBuf::from_str(&self.output_path)
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;

        let item = Job::from(mode, &project_file, blender_version, output)?;
        Ok(WithId { id, item })
    }
}

#[async_trait::async_trait]
impl JobStore for SqliteJobStore {
    async fn add_job(&mut self, job: NewJobDto) -> Result<CreatedJobDto, JobError> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let mode = serde_json::to_string::<RenderMode>(job.as_ref()).unwrap();
        let blend_file = AsRef::<BlendFile>::as_ref(&job).to_path().to_string_lossy();
        let blender_version = serde_json::to_string(AsRef::<Version>::as_ref(&job)).unwrap();
        let output = AsRef::<Output>::as_ref(&job).to_str().unwrap().to_owned();

        sqlx::query!(
            r"
                INSERT INTO jobs (id, mode, project_file, blender_version, output_path)
                VALUES($1, $2, $3, $4, $5);
            ",
            id_str,
            mode,
            blend_file,
            blender_version,
            output
        )
        .execute(&self.conn)
        .await
        .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        Ok(CreatedJobDto { id, item: job })
    }

    async fn get_job(&self, job_id: &Uuid) -> Result<Option<CreatedJobDto>, JobError> {
        let id_str = job_id.to_string();
        match sqlx::query_as!(
            JobDAO,
            r#"SELECT id, mode as "mode: Json<RenderMode>", project_file, blender_version as "blender_version: Json<Version>", output_path FROM Jobs WHERE id=$1"#,
            id_str
        )
        .fetch_optional(&self.conn)
        .await
        {
            Ok(record) => match record {
                Some(r) => {
                    let id = Uuid::parse_str(&r.id).unwrap();
                    let mode: RenderMode = r.mode.into_inner();
                    let project = PathBuf::from(r.project_file);
                    let version = r.blender_version.into_inner();
                    let output = PathBuf::from(r.output_path);
                    match Job::from(mode, &project, version, output) {
                        Ok(job) => Ok(Some(WithId { id, item: job })),
                        Err(e) => Err(JobError::InvalidFile(e.to_string())),
                    }
                }
                None => Ok(None),
            },
            Err(e) => Err(JobError::DatabaseError(e.to_string())),
        }
    }

    async fn update_job(&mut self, job: CreatedJobDto) -> Result<(), JobError> {
        let id = job.id.to_string();
        let item = &job.item;
        let mode = serde_json::to_string(item.into()).unwrap();
        let project = AsRef::<BlendFile>::as_ref(&item)
            .to_path()
            .to_string_lossy();
        let version = AsRef::<Version>::as_ref(&item).to_string();
        let output = AsRef::<Output>::as_ref(&item)
            .to_str()
            .expect("Must have valid path!");

        match sqlx::query!(
            r"UPDATE Jobs SET mode=$2, project_file=$3, blender_version=$4, output_path=$5
            WHERE id=$1",
            id,
            mode,
            project,
            version,
            output
        )
        .execute(&self.conn)
        .await
        {
            Ok(record) => match record.rows_affected() {
                0 => Err(JobError::DatabaseError(
                    "Unable to find record! No record was affected!".into(),
                )),
                1 => Ok(()),
                _ => Err(JobError::DatabaseError(format!(
                    "More than one records was affected! {}",
                    record.rows_affected()
                ))),
            },
            Err(e) => Err(JobError::DatabaseError(e.to_string())),
        }
    }

    async fn list_all(&self) -> Result<Vec<CreatedJobDto>, JobError> {
        let query = query_as!(
            JobDAO,
            r#"SELECT id, mode as "mode: Json<RenderMode>", project_file, blender_version as "blender_version: Json<Version>", output_path FROM jobs LIMIT 20"#
        );

        let result = query.fetch_all(&self.conn).await;
        match result {
            Ok(records) => Ok(records.iter().fold(Vec::new(), |mut record, item| {
                if let Ok(obj) = item.clone().dto_to_obj() {
                    record.push(obj);
                }
                record
            })),
            Err(e) => Err(JobError::DatabaseError(e.to_string())),
        }
    }

    async fn delete_job(&mut self, id: &Uuid) -> Result<(), JobError> {
        if let Err(e) = sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.conn)
            .await
        {
            eprintln!("Fail to delete job! {e:?}");
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::models::job::test::mock_job;
    use crate::{config_sqlite_db, constant::DATABASE_FILE_NAME};
    use std::fs;

    pub(crate) async fn get_sqlite_pool() -> SqlitePool {
        let pool = config_sqlite_db(DATABASE_FILE_NAME).await;
        assert!(pool.is_ok());
        pool.expect("Should be ok")
    }

    async fn scaffold_job_store() -> SqliteJobStore {
        let conn = get_sqlite_pool().await;
        SqliteJobStore::new(conn)
    }

    fn mock_job_dao(blend_path: Option<PathBuf>) -> JobDAO {
        let id = Uuid::new_v4().to_string();
        let mode = Json::from(RenderMode::Frame(1));
        let mock_path =
            fs::canonicalize(PathBuf::from("./")).expect("Should be able to fetch absolute path!");
        // TODO: Must have a valid blend path! job::from will try blend::try_from() to verify integrity of blender files.
        let project_file = match blend_path {
            Some(p) => p.into_string().expect("Should cast into string!"),
            None => mock_path
                .clone()
                .join("./test.blend")
                .into_string()
                .expect("Should cast into string!"),
        };
        let blender_version = Json::from(Version::new(4, 2, 0));
        let output_path = mock_path
            .into_string()
            .expect("Should be able to cast into string!");

        JobDAO {
            id,
            mode,
            project_file,
            blender_version,
            output_path,
        }
    }

    #[test]
    fn assure_dto_to_obj_succeed() {
        let mock = mock_job_dao(None);
        let result = mock.dto_to_obj();
        // We would expect the mock object to fail as we do not know exactly where the .blend file is located.
        // // However, if we can load the blender_rs, we can utilize this unit test to verify that this will pass.
        assert!(result.is_err());
        // TODO: Find a way to provide an actual blend file path to get passing result.
    }

    #[tokio::test]
    async fn assure_add_job_succeed() {
        let mut job_store = scaffold_job_store().await;
        let job = mock_job();

        let result = job_store.add_job(job).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_job_success() {
        let mut job_store = scaffold_job_store().await;
        let job = mock_job();

        // append a job to the database first
        let result = job_store.add_job(job).await;
        assert!(result.is_ok());

        // retrieve the ID from the created job we inserted
        let id = result.expect("Should be safe").id;

        // test and see if we can fetch it.
        let fetch_result = job_store.get_job(&id).await;
        assert!(fetch_result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_job_fail_no_record_found() {
        let job_store = scaffold_job_store().await;

        // generate random uuid that doesn't exist in the databset yet
        let fake_id = Uuid::new_v4();

        // query the result
        let result = job_store.get_job(&fake_id).await;

        // Query should be successful, but should return none
        assert!(result.is_ok_and(|e| e.is_none()));
    }
}
