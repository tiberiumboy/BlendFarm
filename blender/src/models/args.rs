/*
    Developer blog

    - Having done extensive research, Blender have two ways to interface to the program
        1. Through CLI
        2. Through Python API via "bpy" library

    Review online for possible solution to interface blender via CAPI, but was strongly suggested to use a python script instead
    this limits what I can do in term of functionality, but it'll be a good start.
    FEATURE - See if python allows pointers/buffer access to obtain job render progress - Allows node to send host progress result. Possibly viewport network rendering?

    Do note that blender is open source - it's not impossible to create FFI that interfaces blender directly, but rather, there's no support to perform this kind of action.

    BlendFarm code shows that they heavily rely on using python code to perform exact operation.
    Question is, do I want to use their code, or do I want to stick with CLI instead?
    I'll try implement both solution, CLI for version and other basic commands, python for advance features and upgrade?
*/

// May Subject to change.

use crate::models::{device::Device, engine::Engine, format::Format};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Args {
    pub file: PathBuf,          // required
    pub output: PathBuf,        // optional
    pub engine: Engine,         // optional
    pub device: Device,         // optional
    pub format: Format,         // optional - default to Png
    pub use_continuation: bool, // optional - default to false
}

impl Args {
    pub fn new(file: PathBuf, output: PathBuf) -> Self {
        Args {
            file: file,
            output: output,
            // TODO: Change this so that we can properly reflect the engine used by A) Blendfile B) User request, and C) allowlist from machine config
            engine: Default::default(),
            device: Default::default(),
            format: Default::default(),
            use_continuation: false,
        }
    }
}
