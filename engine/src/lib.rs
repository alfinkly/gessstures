mod snapshot;

pub use snapshot::*;

use graph_core::{GraphResource, NodeData, NewContent, NodeIndex};
use physics_core::{PhysicsConfig, PhysicsState, Vec3, tick_physics};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

pub struct Engine {
    pub graph: GraphResource,
    pub physics: PhysicsState,
    pub config: PhysicsConfig,
    pub paused: bool,
    node_order: Vec<NodeIndex>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            graph: GraphResource::default(),
            physics: PhysicsState::new(0),
            config: PhysicsConfig::default(),
            paused: false,
            node_order: Vec::new(),
        }
    }

    pub fn add_content(&mut self, content: NewContent) {
        self.graph.add_node(NodeData {
            id: content.source.clone(),
            label: content.title,
            embedding: None,
            outgoing_links: content.outgoing_links,
        });
    }

    pub fn add_contents(&mut self, contents: Vec<NewContent>) {
        for c in contents {
            self.add_content(c);
        }
    }

    pub fn build_edges(&mut self) {
        let node_ids: Vec<(NodeIndex, String)> = self
            .graph
            .graph
            .node_indices()
            .map(|ni| {
                let data = &self.graph.graph[ni];
                (ni, data.id.clone())
            })
            .collect();

        for (ni, _id) in &node_ids {
            let outgoing: Vec<String> = self.graph.graph[*ni].outgoing_links.clone();
            for link_target in &outgoing {
                for (nj, other_id) in &node_ids {
                    if ni == nj {
                        continue;
                    }
                    let target_file = format!("{}.md", link_target);
                    if other_id.ends_with(&target_file) || other_id.ends_with(link_target) {
                        let exists = self.graph.graph.edges(*ni).any(|e| e.target() == *nj);
                        if !exists {
                            self.graph.add_edge(*ni, *nj, 1.0);
                        }
                    }
                }
            }
        }
    }

    pub fn rebuild_node_order(&mut self) {
        self.node_order = self.graph.graph.node_indices().collect();
        if self.physics.positions.len() != self.node_order.len() {
            let n = self.node_order.len();
            self.physics = PhysicsState::new(n);

            let angle_step = std::f32::consts::TAU / n.max(1) as f32;
            let radius = 5.0;
            for i in 0..n {
                let angle = angle_step * i as f32;
                self.physics.positions[i] = Vec3::new(
                    radius * angle.cos(),
                    radius * angle.sin(),
                    0.0,
                );
            }
        }
    }

    pub fn tick(&mut self) {
        let node_count = self.node_order.len();
        if node_count < 2 {
            return;
        }

        if self.physics.positions.len() != node_count {
            self.rebuild_node_order();
        }

        let edges: Vec<(usize, usize)> = self
            .graph
            .graph
            .edge_references()
            .filter_map(|edge| {
                let from = self.node_order.iter().position(|n| *n == edge.source())?;
                let to = self.node_order.iter().position(|n| *n == edge.target())?;
                Some((from, to))
            })
            .collect();

        let (new_positions, new_velocities) = tick_physics(
            &self.physics.positions,
            &edges,
            &self.physics.velocities,
            &self.config,
            self.paused,
        );

        self.physics.positions = new_positions;
        self.physics.velocities = new_velocities;
    }

    pub fn snapshot(&self) -> GraphSnapshot {
        let nodes: Vec<NodeView> = self
            .node_order
            .iter()
            .enumerate()
            .map(|(i, ni)| {
                let data = &self.graph.graph[*ni];
                let degree = self.graph.graph.edges(*ni).count();
                let pos = if i < self.physics.positions.len() {
                    self.physics.positions[i]
                } else {
                    Vec3::ZERO
                };
                NodeView {
                    index: i,
                    id: data.id.clone(),
                    label: data.label.clone(),
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    degree,
                }
            })
            .collect();

        let edges: Vec<EdgeView> = self
            .graph
            .graph
            .edge_references()
            .filter_map(|edge| {
                let from = self.node_order.iter().position(|n| *n == edge.source())?;
                let to = self.node_order.iter().position(|n| *n == edge.target())?;
                Some(EdgeView {
                    from,
                    to,
                    weight: edge.weight().weight,
                })
            })
            .collect();

        GraphSnapshot { nodes, edges }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_engine_snapshot() {
        let engine = Engine::new();
        let snap = engine.snapshot();
        assert!(snap.nodes.is_empty());
        assert!(snap.edges.is_empty());
    }

    #[test]
    fn engine_with_one_node() {
        let mut engine = Engine::new();
        engine.add_content(NewContent {
            source: "test.md".into(),
            raw_text: "# Hello".into(),
            title: "Hello".into(),
            outgoing_links: vec![],
        });
        engine.rebuild_node_order();
        engine.tick();
        let snap = engine.snapshot();
        assert_eq!(snap.nodes.len(), 1);
        assert_eq!(snap.nodes[0].label, "Hello");
    }

    #[test]
    fn build_edges_links_nodes() {
        let mut engine = Engine::new();
        engine.add_content(NewContent {
            source: "a.md".into(),
            raw_text: "# A".into(),
            title: "A".into(),
            outgoing_links: vec!["b".into()],
        });
        engine.add_content(NewContent {
            source: "b.md".into(),
            raw_text: "# B".into(),
            title: "B".into(),
            outgoing_links: vec![],
        });
        engine.rebuild_node_order();
        engine.build_edges();
        let snap = engine.snapshot();
        assert_eq!(snap.nodes.len(), 2);
        assert!(!snap.edges.is_empty(), "should have one edge from a to b");
    }
}
