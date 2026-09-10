// TODO: make this user adjustable.
// Ideally, this should be store under BlendFarmUserSettings
// pub const MAX_FRAME_CHUNK_SIZE: i32 = 30;

#[cfg(test)]
pub mod test {
    // TODO: Remove this as this is no longer valid in testing environment!
    // Find a way to create or load blend file to be used for unit testing purposes.
    // An idea is to download the blend file from blender.org and use that as a unit test to run against?
    pub const EXAMPLE_FILE: &str = "./../../blender_rs/examples/assets/test.blend";
    pub const EXAMPLE_OUTPUT: &str = "./../../blender_rs/examples/assets/";
}
