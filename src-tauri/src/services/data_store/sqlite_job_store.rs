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
use sqlx::{FromRow, SqlitePool, query_as};
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
    // TODO: Convert this into serde::json?
    mode: String,
    project_file: String,
    // This is Version (major.minor.patch)
    blender_version: String,
    output_path: String,
}

impl JobDAO {
    pub fn dto_to_obj(self) -> Result<WithId<Job, Uuid>, JobError> {
        let id = Uuid::from_str(&self.id).expect("id malformed");
        let mode = serde_json::from_str(&self.mode).expect("mode malformed");
        let project_file = PathBuf::from_str(&self.project_file).expect("Project path malformed");
        let blender_version =
            Version::from_str(&self.blender_version).expect("Blender version malformed");
        let output = PathBuf::from_str(&self.output_path).expect("Output path malformed");
        Job::from(mode, &project_file, blender_version, output).and_then(|item| Ok(WithId { id, item }))
    }
}

#[async_trait::async_trait]
impl JobStore for SqliteJobStore {
    async fn add_job(&mut self, job: NewJobDto) -> Result<CreatedJobDto, JobError> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let mode = serde_json::to_string::<RenderMode>(job.as_ref()).unwrap();
        let blend_file = AsRef::<BlendFile>::as_ref(&job).to_path().to_string_lossy();
        let blender_version = AsRef::<Version>::as_ref(&job).to_string();
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
            r"SELECT id, mode, project_file, blender_version, output_path FROM Jobs WHERE id=$1",
            id_str
        )
        .fetch_optional(&self.conn)
        .await
        {
            Ok(record) => match record {
                Some(r) => {
                    let id = Uuid::parse_str(&r.id).unwrap();
                    let mode: RenderMode = serde_json::from_str(&r.mode).unwrap();
                    let project = PathBuf::from(r.project_file);
                    let version = Version::from_str(&r.blender_version).unwrap();
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
            r"SELECT id, mode, project_file, blender_version, output_path FROM jobs LIMIT 20"
        );

        let result = query.fetch_all(&self.conn).await;
        match result {
            Ok(records) => Ok(records
                .iter()
                .fold( Vec::new(),|mut record, item| {
                    if let Ok(obj) = item.clone().dto_to_obj() {
                        record.push(obj);
                    }
                    record
                })
            ),
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
mod tests {
    use crate::{config_sqlite_db, constant::DATABASE_FILE_NAME, models::job::test::scaffold_job};

    use super::*;

    async fn get_sqlite_pool() -> SqlitePool {
        let pool = config_sqlite_db(DATABASE_FILE_NAME).await;
        assert!(pool.is_ok());
        pool.expect("Should be ok")
    }

    async fn scaffold_job_store() -> SqliteJobStore {
        let conn = get_sqlite_pool().await;
        SqliteJobStore::new(conn)
    }

    #[tokio::test]
    async fn can_create_worker_success() {
        let mut job_store = scaffold_job_store().await;
        let job = scaffold_job();

        let result = job_store.add_job(job).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fetch_job_success() {
        let mut job_store = scaffold_job_store().await;
        let job = scaffold_job();

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
