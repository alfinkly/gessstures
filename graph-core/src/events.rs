#[cfg(feature = "bevy")]
use bevy::prelude::*;

#[cfg_attr(feature = "bevy", derive(Event))]
#[derive(Debug, Clone)]
pub struct NewContent {
    pub source: String,
    pub raw_text: String,
    pub title: String,
    pub outgoing_links: Vec<String>,
}

#[cfg_attr(feature = "bevy", derive(Event))]
#[derive(Debug, Clone)]
pub struct GraphUpdate {
    pub graph: petgraph::stable_graph::StableGraph<crate::NodeData, crate::EdgeData>,
}

#[cfg_attr(feature = "bevy", derive(Event))]
#[derive(Debug, Clone)]
pub struct GraphChanged;

#[cfg_attr(feature = "bevy", derive(Event))]
#[derive(Debug, Clone)]
pub struct CameraCommand {
    pub kind: CameraCommandKind,
}

#[derive(Debug, Clone)]
pub enum CameraCommandKind {
    Orbit { delta_yaw: f32, delta_pitch: f32 },
    Pan { delta_x: f32, delta_y: f32 },
    Zoom(f32),
    Reset,
}

#[cfg_attr(feature = "bevy", derive(Event))]
#[derive(Debug, Clone)]
pub struct GraphQuery {
    pub query: String,
}
