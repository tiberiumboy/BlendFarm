use serde::Serialize;
use sqlx::prelude::*;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct WithId<T: Serialize, ID: Serialize> {
    pub id: ID,
    pub item: T,
}

impl<T> AsRef<Uuid> for WithId<T, Uuid>
where
    T: Serialize,
{
    fn as_ref(&self) -> &Uuid {
        &self.id
    }
}

impl<T> PartialEq<Uuid> for WithId<T, Uuid>
where
    T: Serialize,
{
    fn eq(&self, other: &Uuid) -> bool {
        self.id.eq(other)
    }
}