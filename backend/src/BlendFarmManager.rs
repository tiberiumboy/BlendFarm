use semver::Version;
use RenderNode::RenderNode;

// Todo: Don't think we need a manager? Think abstractness. What exactly do we need to feed to blender?
// BlendFarmManager.cs Ln#74

// currently thinks node aligns, when we need to think like a pool instead.
struct BlendFarmManager {
    pub version: Version,
    pub selected_session_id: i32, // expects guid, using ints instead - can change this anytime after
    pub sessions: vec![BlendFarmFileSession],
    pub nodes: vec![RenderNode], // could potentially be an issue? Let's make it not a dependency, instead make it like a pool?
}

impl BlenderFarmManager {}
