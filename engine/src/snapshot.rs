use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeView {
    pub index: usize,
    pub id: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub degree: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeView {
    pub from: usize,
    pub to: usize,
    pub weight: f32,
}
