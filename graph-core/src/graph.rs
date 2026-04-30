use bevy::prelude::*;
use petgraph::stable_graph::StableGraph;
use crate::{NodeData, EdgeData};

#[derive(Resource, Debug, Clone)]
pub struct GraphResource {
    pub graph: StableGraph<NodeData, EdgeData>,
}

impl Default for GraphResource {
    fn default() -> Self {
        Self {
            graph: StableGraph::new(),
        }
    }
}

impl GraphResource {
    pub fn add_node(&mut self, data: NodeData) -> petgraph::stable_graph::NodeIndex {
        self.graph.add_node(data)
    }

    pub fn add_edge(
        &mut self,
        a: petgraph::stable_graph::NodeIndex,
        b: petgraph::stable_graph::NodeIndex,
        weight: f32,
    ) {
        self.graph.add_edge(a, b, EdgeData { weight });
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}
