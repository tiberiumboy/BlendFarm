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