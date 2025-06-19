use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, FromRow, SqlitePool};
use uuid::Uuid;

use crate::{
    domains::advertise_store::{AdvertiseError, AdvertiseStore},
    models::advertise::Advertise,
};

pub struct SqliteAdvertiseStore {
    conn: SqlitePool,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
struct AdvertiseDAO {
    id: String,
    ad_name: String,
    file_path: String,
}

impl AdvertiseDAO {
    pub fn dto_to_obj(self) -> Advertise {
        let id = Uuid::from_str(&self.id).expect("ID was mutated!");
        let file_path = PathBuf::from_str(&self.file_path).expect("File path was mutated!");
        Advertise {
            id,
            ad_name: self.ad_name,
            file_path,
        }
    }
}

#[async_trait::async_trait]
impl AdvertiseStore for SqliteAdvertiseStore {
    async fn find(&self, id: Uuid) -> Result<Option<Advertise>, AdvertiseError> {
        let id = id.to_string();
        match query_as!(
            AdvertiseDAO,
            r"SELECT id, ad_name, file_path FROM advertise WHERE id=$1",
            id
        )
        .fetch_optional(&self.conn)
        .await
        {
            Ok(dto) => Ok(dto.map(|d| d.dto_to_obj())),
            Err(e) => Err(AdvertiseError::DatabaseError(e.to_string())),
        }
    }

    async fn update(&self, advertise: Advertise) -> Result<(), AdvertiseError> {
        let id = advertise.id.to_string();
        let file_path = advertise.file_path.to_str();
        query!(
            "UPDATE advertise SET ad_name=$2, file_path=$3 WHERE id=$1",
            id,
            advertise.ad_name,
            file_path
        )
        .execute(&self.conn)
        .await
        .map_err(|e| AdvertiseError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn create(&self, advertise: Advertise) -> Result<(), AdvertiseError> {
        let id = advertise.id.to_string();
        let file_path = advertise.file_path.to_str();
        if let Err(e) = query!(
            r"
                INSERT INTO advertise (id, ad_name, file_path)
                VALUES($1, $2, $3);
            ",
            id,
            advertise.ad_name,
            file_path
        )
        .execute(&self.conn)
        .await
        {
            return Err(AdvertiseError::DatabaseError(e.to_string()));
        }

        Ok(())
    }

    async fn kill(&self, id: Uuid) -> Result<(), AdvertiseError> {
        let id = id.to_string();
        let _ = query!(r"DELETE FROM advertise WHERE id=$1", id)
            .execute(&self.conn)
            .await
            .map_err(|e| AdvertiseError::DatabaseError(e.to_string()))?;
        Ok(())
    }
    async fn all(&self) -> Result<Option<Vec<Advertise>>, AdvertiseError> {
        Ok(None)
    }
}
