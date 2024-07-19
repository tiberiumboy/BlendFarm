# Download blender example
This example will download blender with the version passed into arguments and returns the path to blender executables, unpacked.

## Test it!
To run this example, simply run:
```bash
cargo run --example download <version>

// For example, if I want to download Blender 4.1.0
cargo run --example download 4.1.0
Cache file found! Fetching metadata creation date property!
Successfully saved cache file!
Blender downloaded at: "/Users/User/Downloads/blender/Blender4.1/blender-4.1.0-macos-arm64/Blender.app/Contents/MacOS/Blender"
```
The output result will show you where Blender struct is referencing the executable path that is used to pass to argument commands.