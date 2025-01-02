use crate::{domains::job_store::{JobError, JobStore}, models::job::Job};
use std::{path::PathBuf, sync::Arc};
use sqlx::{Pool, Sqlite, PgRow};
use semver::Version;
use tauri_plugin_sql::{Migration, MigrationKind};
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct SqliteJobStore {
    conn : Arc<RwLock<Pool<Sqlite>>>
}

impl SqliteJobStore {
    pub fn get_migration() -> Migration {
        Migration {
            version: 1,
            description: "create_job_table",
            sql: "CREATE TABLE IF NOT EXISTS dbo.jobs (id TEXT UNIQUE, mode BLOB, project_file TEXT, blender_version TEXT, output TEXT)",
            kind: MigrationKind::Up
        }
    }

    pub async fn new(conn: Arc<RwLock<Pool<Sqlite>>>) -> Result<Self, sqlx::Error> {
        // TODO: Run sql migration to see if the job store info exist!
        Ok( Self { conn } )
    }
}

#[async_trait::async_trait]
impl JobStore for SqliteJobStore {
    async fn add_job(&mut self, job: Job) -> Result<(),JobError>  {
        let db = self.conn.write().await;
        let mut pool = db.acquire().await.unwrap().detach();
        let id = job.id.to_string();
        let mode = serde_json::to_string(&job.mode).unwrap();
        let project_file = job.project_file.to_str().unwrap().to_owned();
        let blender_version = job.blender_version.to_string();
        let output = job.output.to_str().unwrap().to_owned();

        sqlx::query(r"
                INSERT INTO jobs (id, mode, project_file, blender_version, output)
                VALUES($1, $2, $3, $4, $5)
            ")
            .bind(id)
            .bind(mode)
            .bind(project_file)
            .bind(blender_version)
            .bind(output)
            .execute(&mut pool)
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn update_job(&mut self, job: Job) -> Result<(), JobError> {
        todo!("Update job to database");
    }

    async fn list_all(&self) -> Result<Vec<Job>, JobError> {
        let mut conn = self.conn.write().await;
        let mut pool = conn.acquire().await.unwrap().detach();
        let mut stream = 
            sqlx::query(r"SELECT * FROM jobs")
            .map(|row:PgRow| { 
                let id = row["id"];
                let mode = serde_json::from_str(row["mode"]).unwrap();
                let project_file = PathBuf::from(row["project_file"]);
                let blender_version = Version::from(row["blender_version"]);
                let output = PathBuf::from(row["output"]);
                Job::new(id, mode, project_file, blender_version, output, Default::default())
            })
            .fetch(&mut pool).await;
        
        Ok(vec![])
    }

    async fn delete_job(&mut self, id: Uuid) -> Result<(), JobError> {
        let mut conn = self.conn.write().await;
        let mut pool = conn.acquire().await.unwrap().detach();
        sqlx::query("DELETE * FROM job WHERE id = $1").bind(id.to_string()).execute(&mut pool).await;
        Ok(())
    }
}