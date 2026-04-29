use bevy::prelude::*;
use petgraph::stable_graph::NodeIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphInteractionMode {
    #[default]
    GraphView,
    VertexPinned,
    VertexContent,
}

#[derive(Resource, Debug, Clone)]
pub struct InteractionState {
    pub mode: GraphInteractionMode,
    pub pinned_node: Option<NodeIndex>,
    pub hovered_node: Option<NodeIndex>,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            mode: GraphInteractionMode::GraphView,
            pinned_node: None,
            hovered_node: None,
        }
    }
}

impl InteractionState {
    pub fn pin_vertex(&mut self, node: NodeIndex) {
        self.mode = GraphInteractionMode::VertexPinned;
        self.pinned_node = Some(node);
    }

    pub fn enter_content(&mut self) {
        self.mode = GraphInteractionMode::VertexContent;
    }

    pub fn go_back(&mut self) {
        self.mode = GraphInteractionMode::GraphView;
        self.pinned_node = None;
    }

    pub fn set_hovered(&mut self, node: Option<NodeIndex>) {
        self.hovered_node = node;
    }
}
