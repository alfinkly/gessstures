use bevy::prelude::*;

#[derive(Event, Debug, Clone)]
pub struct NewContent {
    pub source: String,
    pub raw_text: String,
    pub title: String,
    pub outgoing_links: Vec<String>,
}

#[derive(Event, Debug, Clone)]
pub struct GraphUpdate {
    pub graph: petgraph::stable_graph::StableGraph<crate::NodeData, crate::EdgeData>,
}

#[derive(Event, Debug, Clone)]
pub struct GraphChanged;

#[derive(Event, Debug, Clone)]
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

#[derive(Event, Debug, Clone)]
pub struct GraphQuery {
    pub query: String,
}
