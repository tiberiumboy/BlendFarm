use crate::{
    domains::ticket_store::{TicketError, TicketStore},
    models::{
        ticket::{CreatedTicketDto, Ticket},
        with_id::WithId,
    },
};
use sqlx::{FromRow, SqlitePool, query, query_as, types::Uuid};
use std::{path::PathBuf, str::FromStr};
use semver::Version;

// Is this how we can make this connection arc across threads?
#[derive(Debug)]
pub struct SqliteTicketStore {
    conn: SqlitePool,
}

impl SqliteTicketStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[derive(Debug, Clone, FromRow)]
struct TicketDAO {
    id: String,
    job_id: String,
    blend_path: String,
    blender_version: String,
    // TODO: See why we can't use Frame (i32). Sqlite impose using i64?
    start: i64,
    end: i64,
}

impl TicketDAO {
    fn dto_to_ticket(self) -> WithId<Ticket, Uuid> {
        let id = Uuid::from_str(&self.id).expect("id was mutated");
        let job_id = Uuid::from_str(&self.job_id).expect("job_id was mutated");
        let blend_path = PathBuf::from_str(&self.blend_path).expect("blend path was malformed!");
        let blender_version = Version::from_str(&self.blender_version).expect("Blender version was malformed!");
        let start = self.start as i32;
        let end = self.end as i32;
        let temp_output = PathBuf::new();

        // TODO: Find a way to handle expect()
        let item = Ticket::new(job_id, blend_path, blender_version, temp_output, start, end);
        WithId { id, item }
    }
}

#[async_trait::async_trait]
impl TicketStore for SqliteTicketStore {
    async fn add_ticket(&self, ticket: Ticket) -> Result<CreatedTicketDto, TicketError> {
        // let sql = ;
        let id = Uuid::new_v4();
        let blend_path = ticket.blend_path.to_string_lossy();
        let blender_version = ticket.blender_version.to_string();

        let job_id = AsRef::<Uuid>::as_ref(&ticket).to_string();

        // todo see if there's a better way to handle sqlite query?
        let _ = query!(
            r"INSERT INTO ticket(id, job_id, blend_path, blender_version, start, end) 
            VALUES($1, $2, $3, $4, $5, $6)",
            id,
            job_id,
            blend_path,
            blender_version,
            ticket.start,
            ticket.end
        )
        .execute(&self.conn)
        .await
        .map_err(TicketError::DatabaseError)?;

        Ok(WithId { id, item: ticket })
    }

    // Poll next available task if there any.
    async fn poll_ticket(&self) -> Result<Option<CreatedTicketDto>, TicketError> {
        // fetch next available task to work on
        // TODO: Implement safeguard logic checks to pull only the tickets that haven't complete the range of renders yet.
        let result = query_as!(
            TicketDAO,
            r"SELECT id, job_id, blend_path, blender_version, start, end FROM ticket LIMIT 1"
        )
        .fetch_optional(&self.conn)
        .await
        .map_err(TicketError::DatabaseError)?;
        Ok(result.map(|d| Some(d.dto_to_ticket())).unwrap_or(None))
    }

    async fn list_tickets(&self) -> Result<Option<Vec<CreatedTicketDto>>, TicketError> {
        let result = sqlx::query_as!(
            TicketDAO,
            r"
                SELECT id, job_id, blend_path, blender_version, start, end
                FROM ticket
                LIMIT 10
            "
        )
        .fetch_all(&self.conn)
        .await;

        match result {
            Ok(list) => Ok(Some(list.iter().map(|d| d.clone().dto_to_ticket()).collect())),
            Err(e) => Err(TicketError::DatabaseError(e)),
        }
    }

    async fn delete_ticket(&self, id: &Uuid) -> Result<(), TicketError> {
        let _ = sqlx::query(r"DELETE FROM ticket WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.conn)
            .await;
        Ok(())
    }

    async fn delete_job_ticket(&self, job_id: &Uuid) -> Result<(), TicketError> {
        let _ = sqlx::query(r"DELETE FROM ticket WHERE job_id = $1")
            .bind(job_id.to_string())
            .execute(&self.conn)
            .await;
        Ok(())
    }
}
