Currently unit test fails when scaffolding job entry. The provided path in there doesn't align to match path to the example file within blender_rs directory. 

It would be nice to find a way to get around this or make this explicit accept any file path for unit testing purposes.

I may have to be explicit create fake path within project file struct to allow unit test to continue and operate normally.

Error message: 

thread 'models::task::test::get_next_frame_success' panicked at /Users/megamind/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blend-0.8.0/src/runtime.rs:1346:41:
could not open .blend file: Os { code: 2, kind: NotFound, message: "No such file or directory" }