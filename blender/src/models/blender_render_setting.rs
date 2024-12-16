use super::{
    args::Args, blender_peek_response::BlenderPeekResponse, device::Device, engine::Engine,
    format::Format,
};
use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};
use std::{ops::Range, path::PathBuf};
use uuid::Uuid;

// In the python script, this Window values gets assigned to border of scn.render.border_*
// Here - I'm calling it as window instead.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub x: Range<f32>,
    pub y: Range<f32>,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            x: Range {
                start: 0.0,
                end: 1.0,
            },
            y: Range {
                start: 0.0,
                end: 1.0,
            },
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BlenderRenderSetting {
    #[serde(rename = "TaskID")]
    pub id: Uuid,
    pub output: PathBuf,
    pub scene: String,
    pub camera: String,
    pub cores: usize,
    pub compute_unit: i32,
    #[serde(rename = "FPS")]
    pub fps: u16, // u32 convert into string for xml-rpc. BEWARE!
    pub border: Window,
    pub tile_width: i32,
    pub tile_height: i32,
    pub samples: i32,
    pub width: i32,
    pub height: i32,
    pub engine: i32,
    #[serde(rename = "RenderFormat")]
    pub format: Format,
    // discourage?
    pub crop: bool,
}

impl BlenderRenderSetting {
    fn new(
        output: PathBuf,
        scene: String,
        camera: String,
        compute_unit: Device,
        fps: u16,
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
            scene,
            camera,
            cores: std::thread::available_parallelism().unwrap().get(),
            compute_unit: compute_unit as i32,
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
        }
    }

    pub fn parse_from(args: &Args, info: &BlenderPeekResponse) -> Self {
        let output = args.output.clone();
        let compute_unit = args.device.clone();
        let border = Default::default();
        let engine = args.engine.clone();
        let format = args.format.clone();

        BlenderRenderSetting::new(
            output.to_owned(),
            info.selected_scene.to_owned(),
            info.selected_camera.to_owned(),
            compute_unit.to_owned(),
            info.fps,
            border,
            -1, // I wonder?
            -1, // I wonder?
            info.samples,
            info.render_width,
            info.render_height,
            engine,
            format,
        )
    }
}
