# BlendFarm

## An open source, cli friendly, Network Rendering Service application for Blender.

This project was inspired by this original project - [LogicReinc](https://github.com/LogicReinc/LogicReinc.BlendFarm)
s backburner application, which saved me many hours of rendering projects for school. I took a turn and realize that Blender soar through popularity among the community and industry. As soon as I realized that Blender, out of the box, does not have any solution to support similar functionality as Autodesk backburner, was the moment I realize this was the piece that is still missing from this amazing open-source, industry leading, software. Digging through online, there are few tools out there that provides "good enough", but I felt like there's so much potential waiting to be tapped into that unlocks the powertrain to speed development all the way to production velocity by utilizing network resources.

I humbly present you BlendFarm 2.0, a open-source software completely re-written in Rust from scratch. Thanks to Tauri library, the use of this tool comes into three separate parts - 
## GUI 
For new users and anyone who wants to get things done quickly. Simply run the application. When you run the app on computers that will be used as a rendering farm, simply navigate to the app and run as client instead. This will minimize the app into a service application, and under the traybar, you can monitor and check your render progress.

## CLI 
For those who wish to run the tools on headless server and network farm solution, this tool provide ease of comfort to setup, robust dialogs and information, and thread safety throughout application lifespan.

## Library
.rlib are publicly available and exposed by compiling rust into the library bundle. You can compile the blender package separately and use the codebase to allow your program to interface blender. Or interface to the manager of the toolchain to help prebuild your assembly with out of box template to interface with blender program.   

# Planned
[ ] Pipe Blender's rendering preview
[ ] Node version distribution - to reduce internet traffic to download version from source.

# Limitations
Blender's limitation applies to this project's scope limitation. If a feature is available, or compatibility to run blender on specific platform - this tool will need to reflect and handle those unique situation. Otherwise, this tool follows Blender's programming guideline to ensure backward compatibility for all version available.

## Getting Started

There are several ways to start; the first and easiest would be to download the files and simply run the executable, the second way is to download the source code and compile on your computer to run and start.

### TLDR:

First - Install tauri-cli as this component is needed to run `cargo tauri` command. Run the following command:
`cargo install tauri-cli --version ^2.0.0-rc --locked`

*Note- For windows, you must encapsulate the version in double quotes!

To run Tauri app - run the following command under `/BlendFarm/` directory - `cargo tauri dev`

To run the client app - run the following command under `/BlendFarm/backend/` directory - `cargo run -- -c`

