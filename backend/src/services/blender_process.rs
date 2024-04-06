enum Status {
    Idle,
    Download(BlenderVersion),
    Transfer(Job), // Would like to be able to transfer file?
    Time(i32),
    Completed,
}

struct BlenderProcess {
    pub cmd: String,
    pub args: String,
    pub version: Version,
    pub file: File,
}

impl BlendProcess {
    fn new(blender: &str, args: &str, version: Version, file: &File) -> BlendProcess {
        BlendProcess {
            cmd: blender,
            args,
            version,
            file,
        }
    }

    fn run() -> io::Result {
        Ok(())
    }

    fn resume() {}

    fn cancel() {}
}
