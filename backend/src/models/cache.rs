use chrono::prelude::*;
use serde::{Deserializer, Serializer};
use std::path::Path;

const CACHE_FILE: String = "VersionCache";

#[derive(Deserializer, Serializer)]
struct Cache {
    pub utc: DateTime<Utc>,
    pub versions: Vec<BlenderVersion>,
}

impl Cache {
    fn getCache(&self) -> Option<Cache> {
        if (!Path::new(&CACHE_FILE).exists) {
            return Err("Cache does not exist!");
        }
        let data = fs::read_to_string(&CACHE_FILE);
        let content = data
            .json::<serde_json::Cache>()
            .expect("Unable to parse data!");
        Ok(data)
    }

    fn setCache(&self) {
        let content = serde_json::to_string(&self);
        fs::write(&content, CACHE_FILE);
    }
}
