use semver::Version;
use serde::{Deserialize, Serialize};

// Blender 4.2 introduce a new enum called BLENDER_EEVEE_NEXT, which is currently handle in python file atm.
const EEVEE_SWITCH: Version = Version::new(4, 2, 0);
const EEVEE_OLD: &'static str = "EEVEE";
const EEVEE_NEW: &'static str = "BLENDER_EEVEE_NEXT";
const CYCLES: &'static str = "CYCLES";
const OPTIX: &'static str = "WORKBENCH";

// TODO: Change this so that it's not based on numbers anymore?
#[derive(Debug, Copy, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Engine {
    Cycles = 0,
    #[default]
    Eevee = 1, // Per Blender 4.2.0 this has been renamed to Eevee_next
    OptiX = 3,
}

impl Serialize for Engine {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.
    }
}

impl Engine {
    // the version is required to determine EEVEE usage.
    fn to_string(&self, version: &Version) -> String {
        match self {
            Engine::Cycles => CYCLES.to_owned(),
            Engine::Eevee => match version.ge(&EEVEE_SWITCH) {
                true => EEVEE_NEW,
                false => EEVEE_OLD,
            }
            .to_owned(),
            Engine::OptiX => OPTIX.to_owned(),
        }
    }
}
