mod graph_builder;
mod renderer;
mod physics;
mod camera;
mod input;
mod labels;
mod gesture_detector;
mod gesture_actions;
mod cursor_mapper;
mod nearest_vertex;
mod vertex_highlight;
mod content_viewer;
mod graph_navigation;

mod interface;
mod app;
mod infrastructure;

use clap::{Parser, Subcommand};
use bevy::prelude::*;
use graph_core::{CameraCommand, NewContent, GraphUpdate, GraphQuery, GraphResource, GraphChanged, InteractionState};
use graph_builder::GraphBuilderPlugin;
use renderer::GraphRendererPlugin;
use physics::ForceLayoutPlugin;
use camera::OrbitCameraPlugin;
use input::TextInputPlugin;
use labels::NodeLabelsPlugin;
use infrastructure::camera::capture_adapter::CameraCapturePlugin;
use interface::plugins::hand_tracking_plugin::HandTrackingPlugin;
use interface::plugins::skeleton_renderer_plugin::SkeletonRendererPlugin;
use vertex_highlight::VertexHighlightPlugin;
use gesture_detector::GestureDetectorPlugin;
use gesture_actions::GestureActionPlugin;
use content_viewer::ContentViewerPlugin;
use graph_navigation::GraphNavigationPlugin;
use infrastructure::camera::overlay_adapter::PreviewVideoPlugin;
use infrastructure::ml::mediapipe_sidecar::SidecarPlugin;
use interface::plugins::preview_plugin::PreviewPlugin;

#[derive(Parser)]
#[command(name = "gessstures", about = "3D graph visualization with hand gestures")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch the desktop Bevy application
    Desktop {
        /// Path to a folder of markdown notes
        #[arg(short, long, default_value = ".")]
        notes: String,
    },
    /// Start a WebSocket server (browser extension mode)
    Web {
        #[arg(short, long, default_value = "3030")]
        port: u16,
    },
    /// Run a text query against the graph
    Query {
        /// The query string
        query: String,
        /// Path to a folder of markdown notes
        #[arg(short, long, default_value = ".")]
        notes: String,
    },
    /// Preview mode: webcam + hand skeleton (MediaPipe)
    Preview {
        /// Camera device index (default 0)
        #[arg(short, long, default_value_t = 0)]
        camera: u32,
    },
    /// Voice-controlled mode (future)
    Voice,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Desktop { notes } => {
            let mut app = App::new();
            app.add_plugins(DefaultPlugins)
                .add_event::<NewContent>()
                .add_event::<GraphUpdate>()
                .add_event::<GraphQuery>()
                .add_event::<CameraCommand>()
                .add_event::<GraphChanged>()
                .init_resource::<GraphResource>()
                .init_resource::<InteractionState>()
                .add_plugins((
                    GraphBuilderPlugin,
                    GraphRendererPlugin,
                    ForceLayoutPlugin,
                    OrbitCameraPlugin,
                    TextInputPlugin,
                    NodeLabelsPlugin,
                    CameraCapturePlugin,
                    HandTrackingPlugin,
                ))
                .add_plugins((
                    SidecarPlugin,
                    PreviewVideoPlugin,
                    VertexHighlightPlugin,
                    SkeletonRendererPlugin,
                    GestureDetectorPlugin,
                    GestureActionPlugin,
                    GraphNavigationPlugin,
                    ContentViewerPlugin {
                        notes_dir: notes.clone(),
                    },
                ))
                .add_systems(Update, update_hovered_node)
                .add_systems(
                    Startup,
                    move |mut events: EventWriter<NewContent>| {
                        load_markdown_folder(&notes, &mut events);
                    },
                );

            app.run();
        }
        Commands::Web { port } => {
            println!("Web server mode on port {} (not yet implemented)", port);
        }
        Commands::Query { query, notes } => {
            println!("Query '{}' on notes folder '{}' (not yet implemented)", query, notes);
        }
        Commands::Preview { camera: _cam_idx } => {
            let mut app = App::new();
            app.add_plugins(DefaultPlugins)
               .add_plugins(PreviewPlugin);
            app.run();
        }
        Commands::Voice => {
            println!("Voice mode (not yet implemented)");
        }
    }
}

fn update_hovered_node(
    gesture_state: Res<crate::gesture_detector::GestureState>,
    mut interaction: ResMut<InteractionState>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    node_q: Query<(&Transform, &crate::renderer::GraphNode)>,
) {
    if !gesture_state.hand_detected {
        interaction.set_hovered(None);
        return;
    }

    let Ok((cam, cam_transform)) = camera_q.get_single() else {
        return;
    };

    if let Some(world_pos) = cursor_mapper::map_hand_to_3d(
        gesture_state.cursor_x,
        gesture_state.cursor_y,
        cam_transform,
        cam,
    ) {
        let nearest = nearest_vertex::find_nearest_vertex(world_pos, &node_q, 5.0);
        interaction.set_hovered(nearest.map(|(idx, _)| idx));
    }
}

fn load_markdown_folder(path: &str, events: &mut EventWriter<NewContent>) {
    use walkdir::WalkDir;
    use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};

    let root = std::path::Path::new(path);
    if !root.exists() {
        info!("Notes folder '{}' does not exist, skipping", path);
        return;
    }

    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_path = entry.path();
        if file_path.extension().map_or(false, |ext| ext == "md") {
            let Ok(text) = std::fs::read_to_string(file_path) else {
                continue;
            };

            let relative = file_path.strip_prefix(root).unwrap_or(file_path);
            let source = relative.display().to_string();

            // Parse H1 title and links
            let parser = Parser::new(&text);
            let mut title = String::new();
            let mut md_links: Vec<String> = Vec::new();
            let mut in_h1 = false;
            let mut in_code_block = false;
            let mut title_found = false;

            for event in parser {
                match event {
                    Event::Start(Tag::Heading {
                        level: HeadingLevel::H1,
                        ..
                    }) if !in_code_block => {
                        in_h1 = true;
                    }
                    Event::Text(t) | Event::Code(t) if in_h1 && !in_code_block => {
                        title.push_str(&t);
                        title_found = true;
                    }
                    Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                        in_h1 = false;
                    }
                    Event::Start(Tag::Link { dest_url, .. }) if !in_code_block => {
                        let target = dest_url.to_string();
                        if !target.starts_with("http") && !target.starts_with('#')
                            && target.contains('.')
                        {
                            md_links.push(target);
                        }
                    }
                    Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
                    Event::End(TagEnd::CodeBlock) => in_code_block = false,
                    _ => {}
                }
            }

            // If no H1 found, use filename without extension
            if !title_found {
                title = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&source)
                    .to_string();
            }

            // Sanitize title: truncate and remove garbage
            sanitize_title(&mut title);

            // Parse [[wikilinks]] manually
            let wikilinks = extract_wikilinks(&text);
            md_links.extend(wikilinks);

            events.send(NewContent {
                source,
                raw_text: text,
                title,
                outgoing_links: md_links,
            });
        }
    }
}

fn extract_wikilinks(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len().saturating_sub(1) {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            i += 2;
            let start = i;
            let mut target_end = i;
            while i < bytes.len() {
                if bytes[i] == b']' && i + 1 < bytes.len() && bytes[i + 1] == b']' {
                    target_end = i;
                    i += 2;
                    break;
                }
                // Handle alias separator | and heading #
                if bytes[i] == b'|' || bytes[i] == b'#' {
                    target_end = i;
                }
                i += 1;
            }
            if target_end > start {
                let target = std::str::from_utf8(&bytes[start..target_end])
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !target.is_empty() {
                    links.push(target);
                }
            }
        } else {
            i += 1;
        }
    }
    links
}

/// Truncate title to max 80 visible chars, strip merge conflict markers.
fn sanitize_title(title: &mut String) {
    title.retain(|c| c != '<' && c != '>' && c != '|' && c != '=');

    if title.chars().count() > 80 {
        *title = title.chars().take(80).collect();
    }

    *title = title.trim().to_string();
}
