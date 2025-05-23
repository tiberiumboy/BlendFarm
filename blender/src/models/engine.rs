use serde::{Deserialize, Serialize};
// use semver::Version;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Engine {
    #[allow(non_camel_case_types)]
    BLENDER_EEVEE, // Pre 4.2.0
    #[allow(non_camel_case_types)]
    BLENDER_EEVEE_NEXT, // After 4.2.0
    CYCLES,
    OPTIX,
}
