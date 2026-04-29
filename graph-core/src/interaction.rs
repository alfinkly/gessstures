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

    /// Go back one level in the mode hierarchy:
    /// VertexContent → VertexPinned (preserves pinned_node)
    /// otherwise → GraphView (clears pinned_node)
    pub fn go_back(&mut self) {
        if self.mode == GraphInteractionMode::VertexContent {
            self.mode = GraphInteractionMode::VertexPinned;
            // Stay on same pinned node, just go back to pin view
        } else {
            self.mode = GraphInteractionMode::GraphView;
            self.pinned_node = None;
        }
    }

    /// Navigate to a linked vertex from pinned mode.
    /// Keeps mode in VertexPinned, just changes the pinned node.
    pub fn navigate_to_linked(&mut self, new_node: NodeIndex) {
        self.pinned_node = Some(new_node);
        // stay in VertexPinned mode
    }

    pub fn set_hovered(&mut self, node: Option<NodeIndex>) {
        self.hovered_node = node;
    }
}
