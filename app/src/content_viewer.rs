use bevy::prelude::*;
use graph_core::{GraphInteractionMode, GraphResource, InteractionState};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ContentViewerPlugin {
    pub notes_dir: String,
}

impl Plugin for ContentViewerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NotesDirectory(self.notes_dir.clone()))
            .add_systems(Startup, setup_content_panel)
            .add_systems(Update, update_content_panel);
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct NotesDirectory(String);

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct ContentPanel;

#[derive(Component)]
struct ContentPanelTitle;

#[derive(Component)]
struct ContentPanelPath;

#[derive(Component)]
struct ContentPanelBody;

#[derive(Component, Default)]
struct ContentLoadState {
    last_node_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

fn setup_content_panel(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0),
                width: Val::Px(420.0),
                max_height: Val::Px(640.0),
                padding: UiRect::all(Val::Px(16.0)),
                overflow: Overflow::scroll_y(),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.06, 0.06, 0.10).into()),
            ContentPanel,
            ContentLoadState::default(),
            Name::new("ContentPanel"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 1.0)),
                ContentPanelTitle,
                Name::new("ContentPanelTitle"),
            ));

            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.45, 0.45, 0.65)),
                ContentPanelPath,
                Name::new("ContentPanelPath"),
            ));

            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.3).into()),
                Name::new("ContentPanelSeparator"),
            ));

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    Name::new("ContentPanelScroll"),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.78, 0.78, 0.88)),
                        ContentPanelBody,
                        Name::new("ContentPanelBody"),
                    ));
                });
        });
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

fn update_content_panel(
    interaction: Res<InteractionState>,
    graph: Res<GraphResource>,
    notes_dir: Res<NotesDirectory>,
    mut panel_q: Query<(&mut Node, &mut ContentLoadState), With<ContentPanel>>,
    mut title_q: Query<&mut Text, (With<ContentPanelTitle>, Without<ContentPanelPath>)>,
    mut path_q: Query<&mut Text, (With<ContentPanelPath>, Without<ContentPanelBody>)>,
    mut body_q: Query<&mut Text, With<ContentPanelBody>>,
) {
    let Ok((mut panel_node, mut load_state)) = panel_q.get_single_mut() else {
        return;
    };

    if interaction.mode != GraphInteractionMode::VertexContent {
        panel_node.display = Display::None;
        return;
    }

    panel_node.display = Display::Flex;

    let Some(pinned_node) = interaction.pinned_node else {
        return;
    };

    let node_data = &graph.graph[pinned_node];

    if load_state.last_node_id.as_deref() == Some(&node_data.id) {
        return;
    }

    let file_path = std::path::Path::new(&notes_dir.0).join(&node_data.id);
    let content = std::fs::read_to_string(&file_path).unwrap_or_else(|_| {
        format!("// Unable to read file: {}", file_path.display())
    });

    load_state.last_node_id = Some(node_data.id.clone());

    if let Ok(mut title_text) = title_q.get_single_mut() {
        title_text.0 = node_data.label.clone();
    }
    if let Ok(mut path_text) = path_q.get_single_mut() {
        path_text.0 = node_data.id.clone();
    }
    if let Ok(mut body_text) = body_q.get_single_mut() {
        body_text.0 = content;
    }
}
