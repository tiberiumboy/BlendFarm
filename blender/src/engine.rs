use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Engine {
    Cycles,
    Eevee,
    Workbench,
}

impl ToString for Engine {
    fn to_string(&self) -> String {
        match self {
            Engine::Cycles => "CYCLES".to_owned(),
            Engine::Eevee => "EEVEE".to_owned(),
            Engine::Workbench => "WORKBENCH".to_owned(),
        }
    }
}
