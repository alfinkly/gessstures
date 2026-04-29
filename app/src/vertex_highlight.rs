use bevy::prelude::*;
use graph_core::InteractionState;
use crate::renderer::GraphNode;

#[derive(Component)]
pub struct VertexHighlight {
    pub original_material: Handle<StandardMaterial>,
}

pub struct VertexHighlightPlugin;

impl Plugin for VertexHighlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_vertex_highlights);
    }
}

fn update_vertex_highlights(
    interaction: Res<InteractionState>,
    mut commands: Commands,
    mut node_query: Query<(
        Entity,
        &GraphNode,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        Option<&VertexHighlight>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let hovered = interaction.hovered_node;

    for (entity, graph_node, mut transform, mut material_handle, highlight) in
        node_query.iter_mut()
    {
        let is_hovered = Some(graph_node.0) == hovered;
        let target_scale = if is_hovered { 1.3 } else { 1.0 };
        let current_scale = transform.scale.x;

        if (current_scale - target_scale).abs() > 0.001 {
            let new_scale = current_scale + (target_scale - current_scale) * 0.1;
            transform.scale = Vec3::splat(new_scale);
        } else {
            transform.scale = Vec3::splat(target_scale);
        }

        match (is_hovered, highlight) {
            (true, None) => {
                let original = material_handle.0.clone();
                let highlight_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.3, 0.8, 1.0).into(),
                    emissive: Color::srgb(0.3, 0.5, 1.0).into(),
                    ..default()
                });
                material_handle.0 = highlight_mat;
                commands
                    .entity(entity)
                    .insert(VertexHighlight { original_material: original });
            }
            (false, Some(vh)) => {
                material_handle.0 = vh.original_material.clone();
                commands.entity(entity).remove::<VertexHighlight>();
            }
            _ => {}
        }
    }
}
