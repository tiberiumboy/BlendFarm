use serde::Serialize;
use sqlx::prelude::*;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct WithId<T: Serialize, ID: Serialize> {
    pub id: ID,
    pub item: T,
}

// TODO: Find a way to make this generic as possible without hardcoding for Uuid
impl<T> AsRef<Uuid> for WithId<T, Uuid>
where
    T: Serialize,
{
    fn as_ref(&self) -> &Uuid {
        &self.id
    }
}

// TODO: Find a way to make this generic as possible without hardcoding for Uuid
impl<T> PartialEq<Uuid> for WithId<T, Uuid>
where
    T: Serialize,
{
    fn eq(&self, other: &Uuid) -> bool {
        self.id.eq(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_with_id() -> WithId<String, Uuid> {
        WithId {
            id: Uuid::new_v4(),
            item: "data".to_owned(),
        }
    }

    #[test]
    fn ensure_as_ref_succeed() {
        let record = mock_with_id();
        let result = record.as_ref();
        assert_eq!(result, &record.id);
    }

    #[test]
    fn ensure_partial_eq_succeed() {
        let record = mock_with_id();
        let id = record.id;
        assert_eq!(record, id);
    }
}
