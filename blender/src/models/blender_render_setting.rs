use super::{
    args::Args, blender_peek_response::BlenderPeekResponse, device::Device, engine::Engine,
    format::Format, mode::Mode,
};
use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};
use std::{ops::Range, path::PathBuf};
use uuid::Uuid;

// In the python script, this Window values gets assigned to border of scn.render.border_*
// Here - I'm calling it as window instead.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Window {
    pub x: Range<f32>,
    pub y: Range<f32>,
}

impl Serialize for Window {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Border", 4)?;
        state.serialize_field("X", &self.x.start)?;
        state.serialize_field("X2", &self.x.end)?;
        state.serialize_field("Y", &self.y.start)?;
        state.serialize_field("Y2", &self.y.end)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Window {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct WindowVisitor;

        impl<'de> Visitor<'de> for WindowVisitor {
            type Value = Window;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Border")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let x = seq.next_element()?.unwrap_or(0.0);
                let x2 = seq.next_element()?.unwrap_or(1.0);
                let y = seq.next_element()?.unwrap_or(0.0);
                let y2 = seq.next_element()?.unwrap_or(1.0);
                Ok(Window {
                    x: Range { start: x, end: x2 },
                    y: Range { start: y, end: y2 },
                })
            }
        }

        const FIELDS: &[&str] = &["X", "X2", "Y", "Y2"];
        deserializer.deserialize_struct("Window", FIELDS, WindowVisitor)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlenderRenderSetting {
    #[serde(rename = "TaskID")]
    pub id: Uuid,
    #[serde(rename = "Output")]
    pub output: PathBuf,
    #[serde(rename = "Frame")]
    pub frame: i32,
    #[serde(rename = "Scene")]
    pub scene: String,
    #[serde(rename = "Camera")]
    pub camera: String,
    #[serde(rename = "Cores")]
    pub cores: usize,
    #[serde(rename = "ComputeUnit")]
    pub compute_unit: i32,
    #[serde(rename = "Denoiser")]
    pub denoiser: String,
    #[serde(rename = "FPS")]
    pub fps: u32,
    // hmm worth checking if it can serialize/deserialize this?
    pub border: Window,
    #[serde(rename = "TileWidth")]
    pub tile_width: i32,
    #[serde(rename = "TileHeight")]
    pub tile_height: i32,
    #[serde(rename = "Samples")]
    pub samples: i32,
    #[serde(rename = "Width")]
    pub width: i32,
    #[serde(rename = "Height")]
    pub height: i32,
    #[serde(rename = "Engine")]
    pub engine: i32,
    #[serde(rename = "RenderFormat")]
    pub format: Format,
    // discourage?
    #[serde(rename = "Crop")]
    pub crop: bool,
    #[serde(rename = "Workaround")]
    // TODO: find a better name for this workaround
    pub workaround: bool,
}

impl BlenderRenderSetting {
    #[allow(dead_code)]
    pub fn new(
        output: PathBuf,
        frame: i32,
        scene: String,
        camera: String,
        compute_unit: Device,
        denoiser: String,
        fps: u32,
        border: Window,
        tile_width: i32,
        tile_height: i32,
        samples: i32,
        width: i32,
        height: i32,
        engine: Engine,
        format: Format,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            output,
            frame,
            scene,
            camera,
            cores: std::thread::available_parallelism().unwrap().get(),
            compute_unit: compute_unit as i32,
            denoiser,
            fps,
            border,
            tile_width,
            tile_height,
            samples,
            width,
            height,
            engine: engine as i32,
            format,
            crop: false,
            workaround: false,
        }
    }

    pub fn parse_from(args: Args, info: BlenderPeekResponse) -> Self {
        let frame = match args.mode {
            Mode::Frame(frame) => frame.to_owned(),
            Mode::Animation { start, end: _ } => start.to_owned(),
            _ => 0,
        };
        // it would be nice to get the formatting rules out of this but oh well?
        // this args.output is the only place being used right now. I don't see any reason why I should have this?
        let output = args.output.join(format!("{:0>5}", frame)).to_owned();
        let compute_unit = args.device.clone();
        let border = Window {
            x: Range {
                start: 0.0,
                end: 1.0,
            },
            y: Range {
                start: 0.0,
                end: 1.0,
            },
        };
        let engine = args.engine.clone();
        let format = args.format.clone();

        BlenderRenderSetting::new(
            output.to_owned(),
            frame,
            info.selected_scene.to_owned(),
            info.selected_camera.to_owned(),
            compute_unit.to_owned(),
            info.denoiser.to_owned(),
            info.fps,
            border,
            -1,
            -1,
            info.samples,
            info.render_width,
            info.render_height,
            engine,
            format,
        )
    }
}
