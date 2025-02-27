use std::path::PathBuf;

use crate::{
    domains::render_store::{RenderError, RenderStore},
    models::render_info::{CreatedRenderInfoDto, NewRenderInfoDto, RenderInfo},
};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

pub struct SqliteRenderStore {
    conn: SqlitePool,
}

impl SqliteRenderStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl RenderStore for SqliteRenderStore {
    async fn list_renders(&self) -> Result<Vec<CreatedRenderInfoDto>, RenderError> {
        // query all and list the renders
        let sql = "SELECT id, job_id, frame, render_path FROM renders";
        // TODO: For future impl, Consider looking into Stream and see how we can take advantage of streaming realtime data?
        let col = sqlx::query(sql)
            .map(|row: SqliteRow| {
                let id = row.try_get(0).expect("Missing id column data");
                let job_id = row.try_get(1).expect("Missing job_id column data");
                let frame = row.try_get(2).expect("Missing frame column");
                let render_path: String = row.try_get(3).expect("Missing render_path column");
                let render_path = PathBuf::from(render_path);

                let item = RenderInfo {
                    job_id,
                    frame,
                    render_path,
                };

                CreatedRenderInfoDto { id, item }
            })
            .fetch_all(&self.conn)
            .await
            .map_err(|e| RenderError::DatabaseError(e.to_string()))?;

        Ok(col)
    }

    async fn create_renders(
        &self,
        render_info: NewRenderInfoDto,
    ) -> Result<CreatedRenderInfoDto, RenderError> {
        let sql =
            r#"INSERT INTO renders (id, job_id, frame, render_path) VALUES( $1, $2, $3, $4, $5);"#;
        let id = Uuid::new_v4();
        if let Err(e) = sqlx::query(sql)
            .bind(id.to_string())
            .bind(render_info.job_id.to_string())
            .bind(render_info.frame.to_string())
            .bind(render_info.render_path.to_str())
            .execute(&self.conn)
            .await
        {
            eprintln!("Fail to save data to database! {e:?}");
        }

        Ok(CreatedRenderInfoDto {
            id,
            item: render_info,
        })
    }

    async fn read_renders(&self, id: &Uuid) -> Result<CreatedRenderInfoDto, RenderError> {
        dbg!(id);
        todo!("Impl missing implementations here")
    }

    async fn update_renders(&mut self, render_info: RenderInfo) -> Result<(), RenderError> {
        dbg!(render_info);
        todo!("Impl. missing implementations here")
    }

    async fn delete_renders(&mut self, id: &Uuid) -> Result<(), RenderError> {
        dbg!(id);
        Ok(())
    }
}
