use bevy::prelude::*;
use graph_core::{GraphResource, GraphChanged, NodeIndex};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use physics_core::{PhysicsConfig, Vec3 as PVec3, tick_physics};

pub struct ForceLayoutPlugin;

impl Plugin for ForceLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, force_layout);
    }
}

fn to_pvec3(v: Vec3) -> PVec3 {
    PVec3::new(v.x, v.y, v.z)
}

fn from_pvec3(v: PVec3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn force_layout(
    mut query: Query<(&mut Transform, &crate::renderer::GraphNode)>,
    graph: Res<GraphResource>,
    mut velocities: Local<Vec<PVec3>>,
    mut frame: Local<u64>,
    mut graph_changed: EventWriter<GraphChanged>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let node_count = query.iter().len();
    *frame += 1;

    if node_count < 2 {
        *velocities = vec![];
        return;
    }

    let positions: Vec<(NodeIndex, Vec3)> = query
        .iter()
        .map(|(t, gn)| (gn.0, t.translation))
        .collect();

    if velocities.len() != node_count {
        *velocities = vec![PVec3::ZERO; node_count];
    }

    if *frame % 120 == 0 {
        let centroid: Vec3 = positions.iter().map(|(_, p)| *p).sum::<Vec3>() / node_count as f32;
        info!(
            "Graph: {} nodes, {} edges, centroid at ({:.1}, {:.1}, {:.1})",
            node_count,
            graph.edge_count(),
            centroid.x,
            centroid.y,
            centroid.z,
        );
    }

    let paused = keys.pressed(KeyCode::KeyP);

    let p_positions: Vec<PVec3> = positions.iter().map(|(_, p)| to_pvec3(*p)).collect();
    let edges: Vec<(usize, usize)> = graph
        .graph
        .edge_references()
        .filter_map(|e| {
            let from = positions.iter().position(|(n, _)| *n == e.source())?;
            let to = positions.iter().position(|(n, _)| *n == e.target())?;
            Some((from, to))
        })
        .collect();

    let config = PhysicsConfig::default();
    let (new_positions, new_velocities) = tick_physics(
        &p_positions,
        &edges,
        &velocities,
        &config,
        paused,
    );

    *velocities = new_velocities;

    for (mut transform, graph_node) in &mut query {
        if let Some(i) = positions.iter().position(|(n, _)| *n == graph_node.0) {
            transform.translation = from_pvec3(new_positions[i]);
        }
    }

    graph_changed.send(GraphChanged);
}
