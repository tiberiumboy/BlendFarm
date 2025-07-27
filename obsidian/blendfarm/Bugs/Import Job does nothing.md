When importing a job - I get a log output of this; 

[src/routes/job.rs:40:13] result = Ok(
    WithId {
        id: 78aa6ff7-8bb2-4285-a179-a9bec6407a40,
        item: Job {
            mode: Animation(
                1..10,
            ),
            project_file: ProjectFile {
                inner: "/home/oem/Documents/src/rust/BlendFarm/blender_rs/examples/assets/test.blend",
            },
            blender_version: Version {
                major: 4,
                minor: 4,
                patch: 3,
            },
            output: "/home/oem/Documents/src/rust/BlendFarm/blender_rs/examples/assets",
        },
    },
)

TODO: 
[ ] Update the List to display newly added job user upload
[ ] Send network command out for client to be notify of new jobs available