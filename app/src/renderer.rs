use bevy::prelude::*;
use bevy::color::palettes::css;
use graph_core::{GraphResource, NodeIndex};
use rand::Rng;
use std::f32::consts::{TAU, PI};

#[derive(Component)]
pub struct GraphNode(pub NodeIndex);

#[derive(Component)]
#[allow(dead_code)]
pub struct GraphEdge;

pub struct GraphRendererPlugin;

impl Plugin for GraphRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera, setup_lighting, setup_sphere_meshes))
            .add_systems(Update, (sync_graph_entities, draw_edges));
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_lighting(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
    });
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Resource)]
struct SphereResources {
    mesh_small: Handle<Mesh>,
    mesh_medium: Handle<Mesh>,
    mesh_large: Handle<Mesh>,
    material_default: Handle<StandardMaterial>,
    material_connected: Handle<StandardMaterial>,
    material_hub: Handle<StandardMaterial>,
}

fn setup_sphere_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(SphereResources {
        mesh_small: meshes.add(Sphere::new(0.4)),
        mesh_medium: meshes.add(Sphere::new(0.7)),
        mesh_large: meshes.add(Sphere::new(1.0)),
        material_default: materials.add(StandardMaterial {
            base_color: css::STEEL_BLUE.into(),
            ..default()
        }),
        material_connected: materials.add(StandardMaterial {
            base_color: css::GOLD.into(),
            ..default()
        }),
        material_hub: materials.add(StandardMaterial {
            base_color: css::TOMATO.into(),
            emissive: css::RED.with_alpha(0.3).into(),
            ..default()
        }),
    });
}

fn sync_graph_entities(
    mut commands: Commands,
    graph: Res<GraphResource>,
    sphere_res: Res<SphereResources>,
    existing: Query<(Entity, &GraphNode)>,
) {
    if graph.node_count() == 0 {
        return;
    }

    let existing_nodes: Vec<(usize, Entity)> = existing
        .iter()
        .map(|(e, n)| (n.0.index(), e))
        .collect();

    let _total_nodes = graph.node_count();

    for (_i, node_idx) in graph.graph.node_indices().enumerate() {
        if existing_nodes.iter().any(|(ei, _)| *ei == node_idx.index()) {
            continue;
        }

        // Random position within a small sphere initially
        let mut rng = rand::thread_rng();
        let theta: f32 = rng.gen::<f32>() * TAU;
        let phi: f32 = rng.gen::<f32>() * PI;
        let r: f32 = rng.gen::<f32>() * 4.0;
        let pos = Vec3::new(
            r * phi.sin() * theta.cos(),
            r * phi.cos(),
            r * phi.sin() * theta.sin(),
        );

        // Size and color by degree
        let degree = graph.graph.edges(node_idx).count();
        let (mesh_handle, material_handle) = if degree == 0 {
            (sphere_res.mesh_small.clone(), sphere_res.material_default.clone())
        } else if degree >= 5 {
            (sphere_res.mesh_large.clone(), sphere_res.material_hub.clone())
        } else {
            (sphere_res.mesh_medium.clone(), sphere_res.material_connected.clone())
        };

        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(pos),
            GraphNode(node_idx),
        ));
    }
}

fn draw_edges(
    mut gizmos: Gizmos,
    graph: Res<GraphResource>,
    query: Query<(&Transform, &GraphNode)>,
) {
    let pos_map: Vec<(NodeIndex, Vec3)> = query
        .iter()
        .map(|(t, n)| (n.0, t.translation))
        .collect();

    for edge_idx in graph.graph.edge_indices() {
        if let Some((source, target)) = graph.graph.edge_endpoints(edge_idx) {
            let pos_s = pos_map.iter().find(|(n, _)| *n == source).map(|(_, p)| *p);
            let pos_t = pos_map.iter().find(|(n, _)| *n == target).map(|(_, p)| *p);

            if let (Some(s), Some(t)) = (pos_s, pos_t) {
                gizmos.line(s, t, css::GRAY.with_alpha(0.5));
            }
        }
    }
}
