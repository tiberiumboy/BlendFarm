use crate::{
    domains::ticket_store::{TicketError, TicketStore},
    models::{
        job::Job,
        ticket::{CreatedTaskDto, Ticket},
        with_id::WithId,
    },
};
use sqlx::{FromRow, SqlitePool, query, query_as, types::Uuid};
use std::str::FromStr;

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
    job: String,
    start: i64,
    end: i64,
}

impl TicketDAO {
    fn dto_to_task(self) -> WithId<Ticket, Uuid> {
        let id = Uuid::from_str(&self.id).expect("id was mutated");
        let job_id = Uuid::from_str(&self.job_id).expect("job_id was mutated");
        let job = serde_json::from_str::<Job>(&self.job).expect("job record was malformed!");
        let start = self.start as i32;
        let end = self.end as i32;

        // at this point here, we shouldn't have to worry about Job's original rendering mode,
        let job_record = WithId {
            id: job_id,
            item: job,
        };
        // TODO: Find a way to handle expect()
        let item = Ticket::from(job_record, start, end).expect("Malformed data detected!");
        WithId { id, item }
    }
}

#[async_trait::async_trait]
impl TicketStore for SqliteTicketStore {
    async fn add_task(&self, task: Ticket) -> Result<CreatedTaskDto, TicketError> {
        // let sql = ;
        let id = Uuid::new_v4();
        let job = serde_json::to_string::<Job>(task.as_ref())
            .expect("Should be able to convert job into json");

        let job_id = AsRef::<Uuid>::as_ref(&task).to_string();

        // todo see if there's a better way to handle sqlite query?
        let _ = query!(
            r"INSERT INTO ticket(id, job_id, job, start, end) 
            VALUES($1, $2, $3, $4, $5)",
            id,
            job_id,
            job,
            task.start,
            task.end
        )
        .execute(&self.conn)
        .await
        .map_err(TicketError::DatabaseError)?;

        Ok(WithId { id, item: task })
    }

    // Poll next available task if there any.
    async fn poll_ticket(&self) -> Result<Option<CreatedTaskDto>, TicketError> {
        // fetch next available task to work on
        // TODO: Implement creation date to order by
        let result = query_as!(
            TicketDAO,
            r"SELECT id, job_id, job, start, end FROM ticket LIMIT 1"
        )
        .fetch_optional(&self.conn)
        .await
        .map_err(TicketError::DatabaseError)?;
        Ok(result.map(|d| Some(d.dto_to_task())).unwrap_or(None))
    }

    async fn list_tickets(&self) -> Result<Option<Vec<CreatedTaskDto>>, TicketError> {
        let result = sqlx::query_as!(
            TicketDAO,
            r"
            SELECT id, job_id, job, start, end
            FROM ticket 
            LIMIT 10
        "
        )
        .fetch_all(&self.conn)
        .await;

        match result {
            Ok(list) => Ok(Some(list.iter().map(|d| d.clone().dto_to_task()).collect())),
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
