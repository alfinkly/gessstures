use bevy::prelude::*;
use graph_core::{
    CameraCommand, CameraCommandKind, GraphInteractionMode, GraphResource, InteractionState,
    NodeData, NodeIndex,
};
use hand_tracking_core::Gesture;

use crate::gesture_actions::{GraphAction, GraphActionEvent};
use crate::gesture_detector::GestureState;
use crate::renderer::GraphNode;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GraphNavigationPlugin;

impl Plugin for GraphNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_mode_text)
            .add_systems(Update, (handle_graph_actions, update_mode_text));
    }
}

// ---------------------------------------------------------------------------
// Mode indicator UI
// ---------------------------------------------------------------------------

/// Marker component for the navigation mode text overlay.
#[derive(Component)]
struct ModeText;

/// Returns a human-readable label for the gesture-based navigation mode.
fn mode_label(gesture: Gesture) -> &'static str {
    match gesture {
        Gesture::OpenPalm => "Pan",
        Gesture::Fist => "Orbit",
        Gesture::Pinch => "Pinch",
        Gesture::Point => "Point",
        Gesture::VSIGN => "V-Sign",
        Gesture::Movement => "Navigate",
        Gesture::Unknown => "\u{2014}",
    }
}

fn spawn_mode_text(mut commands: Commands) {
    commands.spawn((
        Text::new("Mode: \u{2014}"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        ModeText,
    ));
}

fn update_mode_text(
    gesture_state: Res<GestureState>,
    mut query: Query<&mut Text, With<ModeText>>,
) {
    let Ok(mut text) = query.get_single_mut() else {
        return;
    };
    text.0 = format!("Mode: {}", mode_label(gesture_state.current_gesture));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Normalize a node id / link target for comparison:
/// strips the `.md` extension and lowercases.
fn normalize_id(id: &str) -> String {
    std::path::Path::new(id)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(id)
        .to_lowercase()
}

/// Given the outgoing_links of the currently pinned node and a vertical
/// hand-movement delta, cycle through linked vertices that actually exist
/// in the graph.
fn navigate_links(
    outgoing_links: &[String],
    delta_y: f32,
    graph: &GraphResource,
    interaction: &mut InteractionState,
) {
    // Find which linked nodes exist in the graph using the same
    // normalisation as build_edges_from_links.
    let linked_nodes: Vec<NodeIndex> = graph
        .graph
        .node_indices()
        .filter_map(|idx| {
            let data: &NodeData = &graph.graph[idx];
            let normalised_id = normalize_id(&data.id);
            if outgoing_links
                .iter()
                .any(|link| normalize_id(link) == normalised_id)
            {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    if linked_nodes.is_empty() {
        return;
    }

    // Cycle through links based on delta_y direction.
    let pos = linked_nodes
        .iter()
        .position(|idx| Some(*idx) == interaction.pinned_node);

    let new_pos = if delta_y > 0.0 {
        // Move forward / next
        match pos {
            Some(p) => (p + 1) % linked_nodes.len(),
            None => 0,
        }
    } else {
        // Move backward / previous
        match pos {
            Some(p) => (p + linked_nodes.len() - 1) % linked_nodes.len(),
            None => 0,
        }
    };

    if new_pos < linked_nodes.len() {
        let new_idx = linked_nodes[new_pos];
        interaction.navigate_to_linked(new_idx);
        interaction.set_hovered(Some(new_idx));
        info!(
            "Navigated through links: pinned vertex now {:?}",
            new_idx
        );
    }
}

// ---------------------------------------------------------------------------
// Core system
// ---------------------------------------------------------------------------

/// Reads `GraphActionEvent`s and translates them into camera commands and
/// interaction-state transitions.
pub(crate) fn handle_graph_actions(
    mut action_events: EventReader<GraphActionEvent>,
    mut interaction: ResMut<InteractionState>,
    graph: Res<GraphResource>,
    mut camera_cmd: EventWriter<CameraCommand>,
    _node_q: Query<(&Transform, &GraphNode)>,
) {
    for event in action_events.read() {
        match &event.action {
            GraphAction::Pan { delta_x, delta_y } => {
                if interaction.mode == GraphInteractionMode::GraphView {
                    camera_cmd.send(CameraCommand {
                        kind: CameraCommandKind::Pan {
                            delta_x: *delta_x,
                            delta_y: *delta_y,
                        },
                    });
                }
            }
            GraphAction::Orbit {
                delta_yaw,
                delta_pitch,
            } => {
                if interaction.mode == GraphInteractionMode::GraphView {
                    camera_cmd.send(CameraCommand {
                        kind: CameraCommandKind::Orbit {
                            delta_yaw: *delta_yaw,
                            delta_pitch: *delta_pitch,
                        },
                    });
                }
            }
            GraphAction::Zoom { amount } => {
                if interaction.mode == GraphInteractionMode::GraphView {
                    camera_cmd.send(CameraCommand {
                        kind: CameraCommandKind::Zoom(*amount),
                    });
                }
            }
            GraphAction::Navigate {
                delta_x,
                delta_y,
                cursor_x: _,
                cursor_y: _,
            } => match interaction.mode {
                GraphInteractionMode::GraphView => {
                    // Orbit camera based on hand movement
                    camera_cmd.send(CameraCommand {
                        kind: CameraCommandKind::Orbit {
                            delta_yaw: delta_x * 3.0,
                            delta_pitch: -delta_y * 3.0,
                        },
                    });
                }
                GraphInteractionMode::VertexPinned => {
                    // Navigate through outgoing links
                    if let Some(pinned) = interaction.pinned_node {
                        if let Some(node_data) = graph.graph.node_weight(pinned) {
                            if delta_y.abs() > 0.01 {
                                navigate_links(
                                    &node_data.outgoing_links,
                                    *delta_y,
                                    &graph,
                                    &mut interaction,
                                );
                            }
                        }
                    }
                }
                GraphInteractionMode::VertexContent => {
                    // No navigation in content mode
                }
            },
            GraphAction::PinVertex => {
                match interaction.mode {
                    GraphInteractionMode::GraphView => {
                        if let Some(node) = interaction.hovered_node {
                            interaction.pin_vertex(node);
                            info!("Pinned vertex {:?}", node);
                        }
                    }
                    GraphInteractionMode::VertexPinned => {
                        // Pinch on a linked vertex → navigate to it
                        if let Some(node) = interaction.hovered_node {
                            interaction.navigate_to_linked(node);
                            info!("Navigated to linked vertex {:?}", node);
                        }
                    }
                    GraphInteractionMode::VertexContent => {
                        // No pinning in content mode
                    }
                }
            }
            GraphAction::ReadContent => {
                if interaction.mode == GraphInteractionMode::VertexPinned {
                    interaction.enter_content();
                    info!("Entering content mode");
                }
            }
            GraphAction::ExitPin => match interaction.mode {
                GraphInteractionMode::VertexPinned => {
                    interaction.go_back(); // → GraphView
                    info!("Exiting pinned mode to graph view");
                }
                GraphInteractionMode::VertexContent => {
                    interaction.go_back(); // → VertexPinned
                    info!("Exiting content mode to pinned view");
                }
                _ => {}
            },
        }
    }
}
