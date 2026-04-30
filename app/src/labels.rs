use bevy::prelude::*;
use crate::renderer::GraphNode;

pub struct NodeLabelsPlugin;

impl Plugin for NodeLabelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_label_ui)
            .add_systems(Update, (sync_label_entities, update_label_positions));
    }
}

#[derive(Component)]
struct NodeLabel {
    node_idx: petgraph::stable_graph::NodeIndex,
}

fn setup_label_ui(mut commands: Commands) {
    // Spawn a root UI node to hold labels
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Name::new("LabelRoot"),
    ));
}

fn sync_label_entities(
    mut commands: Commands,
    graph: Res<graph_core::GraphResource>,
    label_query: Query<(Entity, &NodeLabel)>,
    node_query: Query<(Entity, &GraphNode)>,
    ui_root: Query<Entity, (With<Node>, Without<NodeLabel>)>,
) {
    let Ok(root_entity) = ui_root.get_single() else {
        return;
    };

    let existing_labels: Vec<usize> = label_query
        .iter()
        .map(|(_, l)| l.node_idx.index())
        .collect();

    for (_, graph_node) in &node_query {
        if existing_labels.contains(&graph_node.0.index()) {
            continue;
        }

        let node_data = &graph.graph[graph_node.0];
        let label_text = if node_data.label.chars().count() > 40 {
            format!("{}…", node_data.label.chars().take(37).collect::<String>())
        } else {
            node_data.label.clone()
        };

        let child = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Text::new(label_text),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                NodeLabel {
                    node_idx: graph_node.0,
                },
                Name::new(format!("label-{}", node_data.id)),
            ))
            .id();

        commands.entity(root_entity).add_child(child);
    }
}

fn update_label_positions(
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
    node_query: Query<(&GlobalTransform, &GraphNode)>,
    mut label_query: Query<(&NodeLabel, &mut Node)>,
) {
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };
    let window = windows.single();

    let cam_forward = camera_transform.forward();
    let cam_pos = camera_transform.translation();

    for (label, mut style) in &mut label_query {
        if let Some(node_transform) = node_query
            .iter()
            .find(|(_, gn)| gn.0 == label.node_idx)
            .map(|(t, _)| t)
        {
            let world_pos = node_transform.translation() + Vec3::Y * 0.8;
            
            // Check if node is in front of the camera
            let to_node = world_pos - cam_pos;
            if to_node.dot(*cam_forward) <= 0.0 {
                style.display = Display::None;
                continue;
            }

            if let Ok(viewport_coords) =
                camera.world_to_viewport(camera_transform, world_pos)
            {
                let x = viewport_coords.x * window.width();
                let y = (1.0 - viewport_coords.y) * window.height();

                let in_bounds = x >= -100.0
                    && x <= window.width() + 100.0
                    && y >= -100.0
                    && y <= window.height() + 100.0;

                if in_bounds {
                    style.left = Val::Px(x - 40.0);
                    style.top = Val::Px(y - 8.0);
                    style.display = Display::Flex;
                } else {
                    style.display = Display::None;
                }
            } else {
                style.display = Display::None;
            }
        }
    }
}
