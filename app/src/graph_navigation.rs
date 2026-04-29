use bevy::prelude::*;
use graph_core::{CameraCommand, CameraCommandKind, GraphInteractionMode, InteractionState};

use crate::gesture_actions::{GraphAction, GraphActionEvent};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GraphNavigationPlugin;

impl Plugin for GraphNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_graph_actions);
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
    mut camera_cmd: EventWriter<CameraCommand>,
) {
    for event in action_events.read() {
        match &event.action {
            GraphAction::Navigate {
                delta_x,
                delta_y,
                cursor_x: _,
                cursor_y: _,
            } => {
                // In GraphView: orbit camera based on hand movement
                if interaction.mode == GraphInteractionMode::GraphView {
                    camera_cmd.send(CameraCommand {
                        kind: CameraCommandKind::Orbit {
                            delta_yaw: delta_x * 3.0,
                            delta_pitch: -delta_y * 3.0,
                        },
                    });
                }
                // In VertexPinned: navigate through links (handled by T15)
            }
            GraphAction::PinVertex => {
                if let Some(node) = interaction.hovered_node {
                    if interaction.mode == GraphInteractionMode::GraphView {
                        interaction.pin_vertex(node);
                        info!("Pinned vertex {:?}", node);
                    } else if interaction.mode == GraphInteractionMode::VertexPinned {
                        // Already pinned — pinch again enters content mode
                        interaction.enter_content();
                        info!("Entering content mode from VertexPinned via pinch");
                    }
                }
            }
            GraphAction::ReadContent => {
                if interaction.mode == GraphInteractionMode::VertexPinned {
                    interaction.enter_content();
                    info!("Entering content mode");
                }
            }
            GraphAction::ExitPin => {
                if interaction.mode != GraphInteractionMode::GraphView {
                    interaction.go_back();
                    info!("Exiting to graph view");
                }
            }
        }
    }
}
