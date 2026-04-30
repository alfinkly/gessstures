use bevy::prelude::*;
use graph_core::{GraphResource, GraphChanged, NodeIndex};
use petgraph::visit::EdgeRef;

pub struct ForceLayoutPlugin;

impl Plugin for ForceLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, force_layout);
    }
}

const REPULSION_STRENGTH: f32 = 2.0;
const SPRING_STRENGTH: f32 = 0.3;
const SPRING_REST: f32 = 8.0;
const CENTER_STRENGTH: f32 = 0.03;
const DAMPING: f32 = 0.97;
const MAX_SPEED: f32 = 0.12;
const MIN_DIST: f32 = 0.5;
const BOUNDARY: f32 = 30.0;

fn force_layout(
    mut query: Query<(&mut Transform, &crate::renderer::GraphNode)>,
    graph: Res<GraphResource>,
    mut velocities: Local<Vec<Vec3>>,
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

    let mut positions: Vec<(NodeIndex, Vec3)> = Vec::with_capacity(node_count);
    for (t, gn) in &query {
        positions.push((gn.0, t.translation));
    }

    if velocities.len() != node_count {
        *velocities = vec![Vec3::ZERO; node_count];
    }

    // Status log every 120 frames (~2 sec)
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

    // P = pause physics
    if keys.pressed(KeyCode::KeyP) {
        return;
    }

    for i in 0..positions.len() {
        let (ni, pos_i) = positions[i];

        // Centering
        let dist_center = pos_i.length();
        if dist_center > 0.01 {
            velocities[i] += -pos_i.normalize() * dist_center * CENTER_STRENGTH;
        }

        // Hard boundary bounce
        if dist_center > BOUNDARY {
            velocities[i] += -pos_i.normalize() * (dist_center - BOUNDARY) * 1.5;
        }

        // Repulsion from all others
        for j in 0..positions.len() {
            if i == j {
                continue;
            }
            let (_, pos_j) = positions[j];
            let dir = pos_i - pos_j;
            let dist = dir.length().max(MIN_DIST);
            velocities[i] += dir.normalize_or_zero() * REPULSION_STRENGTH / (dist * dist);
        }

        // Spring attraction along edges
        for edge in graph.graph.edges(ni) {
            let target = edge.target();
            if let Some(j) = positions.iter().position(|(n, _)| *n == target) {
                let (_, pos_j) = positions[j];
                let dir = pos_j - pos_i;
                let dist = dir.length().max(MIN_DIST);
                let force = SPRING_STRENGTH * (dist - SPRING_REST);
                velocities[i] += dir.normalize_or_zero() * force;
            }
        }
    }

    // Damping + clamp
    for v in velocities.iter_mut() {
        *v *= DAMPING;
        let s = v.length();
        if s > MAX_SPEED {
            *v = v.normalize() * MAX_SPEED;
        }
    }

    // Apply
    for (mut transform, graph_node) in &mut query {
        if let Some(i) = positions.iter().position(|(n, _)| *n == graph_node.0) {
            transform.translation += velocities[i];
        }
    }

    graph_changed.send(GraphChanged);
}
