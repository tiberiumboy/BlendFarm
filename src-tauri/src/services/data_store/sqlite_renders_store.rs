use std::{collections::HashMap, path::PathBuf};

use crate::{
    domains::render_store::{RenderError, RenderStore},
    models::{job::JobId, render_info::{CreatedRenderInfoDto, NewRenderInfoDto, RenderInfo}, with_id::WithId},
};
use blender_rs::blender::Frame;
use sqlx::{SqlitePool, query_as};
use uuid::Uuid;

pub struct SqliteRenderStore {
    conn: SqlitePool,
}

impl SqliteRenderStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[derive(Clone)]
struct RenderDAO {
    id: String,
    job_id: String,
    frame: i64,
    render_path: String,
}

impl RenderDAO {
    pub fn to_record(&self) -> Result<WithId<RenderInfo, Uuid>, RenderError> {
        let id = Uuid::parse_str(&self.id).map_err(|e| RenderError::DatabaseError(e.to_string()))?;
        let job_id = Uuid::parse_str(&self.job_id).map_err(|e| RenderError::DatabaseError(e.to_string()))?;
        let render_path = PathBuf::from(&self.render_path);

        let render_info = RenderInfo::new(job_id, self.frame as i32, render_path);
        Ok( WithId { id, item: render_info })
    }
}

#[async_trait::async_trait]
impl RenderStore for SqliteRenderStore {
    async fn find(&self, filter: Option<JobId>) -> Result<HashMap<Frame, PathBuf>, RenderError> {
        // query all and list the renders

        let col = match filter {
            Some(job_id) => {
                query_as!(
                        RenderDAO,
                        r"SELECT id, job_id, frame, render_path FROM renders WHERE job_id=$1",
                        job_id
                    )
                    .fetch_all(&self.conn)
                    .await
                    .map_err(|e| RenderError::DatabaseError(e.to_string()))?
            }
            None => 
                query_as!(
                    RenderDAO,
                    "SELECT id, job_id, frame, render_path FROM renders",
                )
                .fetch_all(&self.conn)
                .await
                .map_err(|e| RenderError::DatabaseError(e.to_string()))?
        }.iter().fold(HashMap::new(),|mut map, item| {
            if let Ok( record ) = &item.to_record() {
                map.insert(record.item.frame, record.item.render_path.clone());
            }

            map
        });
        
        // TODO: For future impl, Consider looking into Stream and see how we can take advantage of streaming realtime data?

        Ok(col)
    }

    async fn create(
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

    async fn update(&mut self, render_info: RenderInfo) -> Result<(), RenderError> {
        dbg!(render_info);
        todo!("Impl. missing implementations here")
    }

    async fn kill(&mut self, id: &Uuid) -> Result<(), RenderError> {
        dbg!(id);
        Ok(())
    }
}
