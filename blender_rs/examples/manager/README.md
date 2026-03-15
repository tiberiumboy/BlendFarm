# Manager example
This example will demonstrate basic cli interface to the manager struct. The manager class requires a path to store configuration file, and load persistent storage. By default it will create one in your application config directory, under the subfolder "BlendFarm". This location will contain a config file, page cache, and render cache. 
blender with the version passed into arguments and returns the path to blender executables, unpacked.

## Test it!
To run this example, simply run:
```bash
# to list installed blenders
cargo run --example manager 

# or update manager with provided installation.
cargo run --example manager add ~/Downloads/Blender-5.0/blender 
```

# Download blender example
This example will download blender with the version passed into arguments and returns the path to blender executables, unpacked, and ready to be use!

## Test it!
To run this example, simply run:
```bash
cargo run --example manager exact-version <version>

// For example, if I want to download Blender 4.1.0
cargo run --example manager exact-download 4.1.0
[Success] Blender 4.1.0 installed at "~/Downloads/Blender/Blender4.1/blender-4.1.0-macos-arm64/Blender.app/Contents/MacOS/Blender"
```
The output result will show you where Blender struct is referencing the executable path that is used to pass to argument commands.