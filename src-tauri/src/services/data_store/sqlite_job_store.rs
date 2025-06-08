use std::{path::PathBuf, str::FromStr};

use crate::{
    domains::job_store::{JobError, JobStore},
    models::{
        job::{CreatedJobDto, Job, NewJobDto},
        with_id::WithId,
    },
};
use blender::models::mode::RenderMode;
use semver::Version;
use sqlx::{query_as, FromRow, SqlitePool};
use uuid::Uuid;

pub struct SqliteJobStore {
    conn: SqlitePool,
}

impl SqliteJobStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

// this information is used to help transcribe the data into database acceptable format.
#[derive(Debug, Clone, FromRow)]
struct JobDAO {
    id: String,
    mode: String,
    project_file: String,
    blender_version: String,
    output_path: String,
}

impl JobDAO {
    pub fn dto_to_obj(self) -> WithId<Job, Uuid> {
        let id = Uuid::from_str(&self.id).expect("id malformed");
        let mode = serde_json::from_str(&self.mode).expect("mode malformed");
        let project_file = PathBuf::from_str(&self.project_file).expect("Project path malformed");
        let blender_version =
            Version::from_str(&self.blender_version).expect("Blender version malformed");
        let output = PathBuf::from_str(&self.output_path).expect("Output path malformed");
        let item = Job::new(mode, project_file, blender_version, output);
        WithId { id, item }
    }
}

#[async_trait::async_trait]
impl JobStore for SqliteJobStore {
    async fn add_job(&mut self, job: NewJobDto) -> Result<CreatedJobDto, JobError> {
        let id = Uuid::new_v4();
        let mode = serde_json::to_string(&job.mode).unwrap();
        let project_file = job.project_file.to_str().unwrap().to_owned();
        let blender_version = job.blender_version.to_string();
        let output = job.output.to_str().unwrap().to_owned();

        sqlx::query(
            r"
                INSERT INTO jobs (id, mode, project_file, blender_version, output_path)
                VALUES($1, $2, $3, $4, $5);
            ",
        )
        .bind(id.to_string())
        .bind(mode)
        .bind(project_file)
        .bind(blender_version)
        .bind(output)
        .execute(&self.conn)
        .await
        .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        Ok(CreatedJobDto { id, item: job })
    }

    async fn get_job(&self, job_id: &Uuid) -> Result<CreatedJobDto, JobError> {
        let sql =
            "SELECT id, mode, project_file, blender_version, output_path FROM Jobs WHERE id=$1";
        match sqlx::query_as::<_, JobDAO>(sql)
            .bind(job_id.to_string())
            .fetch_one(&self.conn)
            .await
        {
            Ok(r) => {
                let id = Uuid::parse_str(&r.id).unwrap();
                let mode: RenderMode = serde_json::from_str(&r.mode).unwrap();
                let project = PathBuf::from(r.project_file);
                let version = Version::from_str(&r.blender_version).unwrap();
                let output = PathBuf::from(r.output_path);
                let item = Job::new(mode, project, version, output);

                Ok(CreatedJobDto { id, item })
            }
            Err(e) => Err(JobError::DatabaseError(e.to_string())),
        }
    }

    async fn update_job(&mut self, job: Job) -> Result<(), JobError> {
        dbg!(job);
        todo!("Update job to database");
    }

    async fn list_all(&self) -> Result<Vec<CreatedJobDto>, JobError> {
        let query = query_as!(
            JobDAO,
            r"
            SELECT id, mode, project_file, blender_version, output_path
            FROM jobs
            LIMIT 10
            "
        );
        // let query = sqlx::query_as::<_, JobDAO>(sql);

        let result = query.fetch_all(&self.conn).await;
        match result {
            Ok(records) => Ok(records.iter().map(|r| r.clone().dto_to_obj()).collect()),
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
