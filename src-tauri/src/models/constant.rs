// TODO: make this user adjustable.
// Ideally, this should be store under BlendFarmUserSettings
// pub const MAX_FRAME_CHUNK_SIZE: i32 = 30;

#[cfg(test)]
pub mod test {
    // TODO: Remove this as this is no longer valid in testing environment!
    pub const EXAMPLE_FILE: &str = "./../../blender_rs/examples/assets/test.blend";
    pub const EXAMPLE_OUTPUT: &str = "./../../blender_rs/examples/assets/";
}
