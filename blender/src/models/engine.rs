use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Engine {
    Cycles = 0,
    #[default]
    Eevee = 1,
    OptiX = 2,
}

impl ToString for Engine {
    fn to_string(&self) -> String {
        match self {
            Engine::Cycles => "CYCLES".to_owned(),
            Engine::Eevee => "EEVEE".to_owned(),
            // TODO: Is this correct?
            Engine::OptiX => "WORKBENCH".to_owned(),
        }
    }
}
