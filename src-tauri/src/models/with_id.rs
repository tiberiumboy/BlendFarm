use serde::Serialize;
use sqlx::prelude::*;
use uuid::Uuid;

use super::network::PeerIdString;

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

impl<T> AsRef<PeerIdString> for WithId<T, PeerIdString> 
where 
    T: Serialize,
{
    fn as_ref(&self) -> &PeerIdString {
        &self.id
    }   
}

impl <T> PartialEq<PeerIdString> for WithId<T, PeerIdString>
where 
    T: Serialize,
{
    fn eq(&self, other: &PeerIdString) -> bool {
        self.id.inner.eq(&other.inner)
    }
}