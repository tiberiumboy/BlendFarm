# BlendFarm

## An open source, cli friendly, Network Rendering Service application for Blender.

This project was inspired by this original project - [LogicReinc](https://github.com/LogicReinc/LogicReinc.BlendFarm)

I discovered Autodesk' backburner application, which saved me many hours of rendering projects for school. I took a turn and realize that Blender soar through popularity among the community and industry. As soon as I realized that Blender, out of the box, does not have any solution to support network rendering farm solution. Digging in a few tools in the community, it provided just a good enough, but felt like it can be better.

I present you BlendFarm 2.0, completely re-written in Rust for the backend service, Tauri for the front end platform, and React for the GUI interface that you see here. 

# Planned
[ ] Pipe Blender's rendering preview
[ ] Node version distribution - to reduce internet traffic to download version from source.

# Limitations
Blender's limitation applies to this project's scope limitation. If a feature is available, or compatibility to run blender on specific platform - this tool will need to reflect and handle those unique situation. Otherwise, this tool follows Blender's programming guideline to ensure backward compatibility for all version available.

## Getting Started

There are several ways to start; the first and easiest would be to download the files and simply run the executable, the second way is to download the source code and compile on your computer to run and start.