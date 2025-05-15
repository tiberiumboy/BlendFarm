use semver::Version;
use serde::{Deserialize, Serialize};

// Blender 4.2 introduce a new enum called BLENDER_EEVEE_NEXT, which is currently handle in python file atm.
const EEVEE_SWITCH: Version = Version::new(4, 2, 0);
const EEVEE_OLD: &'static str = "EEVEE";
const EEVEE_NEW: &'static str = "BLENDER_EEVEE_NEXT";

// TODO: Change this so that it's not based on numbers anymore?
#[derive(Debug, Copy, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Engine {
    CYCLES,
    #[default]
    #[allow(non_camel_case_types)]
    BLENDER_EEVEE, // Per Blender 4.2.0 this has been renamed to "BLENDER_EEVEE_NEXT" instead of "BLENDER_EEVEE"
    OPTIX,
}
