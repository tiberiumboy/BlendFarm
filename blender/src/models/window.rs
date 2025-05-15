use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};
use std::ops::Range;

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

// TODO: Remove this as this may no longer be needed
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
