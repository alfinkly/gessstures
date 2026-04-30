use bevy::prelude::*;
use graph_core::{NewContent, GraphResource, GraphUpdate, NodeData};
use petgraph::stable_graph::NodeIndex;
use std::collections::HashMap;

pub struct GraphBuilderPlugin;

impl Plugin for GraphBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ingest_new_content);
    }
}

pub fn ingest_new_content(
    mut events: EventReader<NewContent>,
    mut graph: ResMut<GraphResource>,
    mut graph_update: EventWriter<GraphUpdate>,
) {
    let mut changed = false;

    for content in events.read() {
        let node = NodeData {
            id: content.source.clone(),
            label: content.title.clone(),
            embedding: None,
            outgoing_links: content.outgoing_links.clone(),
        };
        graph.add_node(node);
        changed = true;
    }

    if changed {
        build_edges_from_links(&mut graph);
        graph_update.send(GraphUpdate {
            graph: graph.graph.clone(),
        });
    }
}

fn build_edges_from_links(graph: &mut GraphResource) {
    let node_count = graph.node_count();
    if node_count < 2 {
        return;
    }

    // Build lookup: various possible path forms → NodeIndex
    let mut lookup: HashMap<String, NodeIndex> = HashMap::new();

    let nodes: Vec<(NodeIndex, String)> = graph
        .graph
        .node_indices()
        .map(|ni| {
            let data = &graph.graph[ni];
            (ni, data.id.clone())
        })
        .collect();

    for (ni, path) in &nodes {
        // Store by full path
        lookup.insert(path.clone(), *ni);
        // Store by filename without extension
        let stem = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_lowercase();
        lookup.entry(stem).or_insert(*ni);
        // Store by relative path (without .md)
        let no_ext = path.trim_end_matches(".md").to_lowercase();
        lookup.entry(no_ext).or_insert(*ni);
    }

    // For each node, create edges based on outgoing links
    for (ni, _path) in &nodes {
        let outgoing: Vec<String> = graph.graph[*ni].outgoing_links.clone();
        for link_target in outgoing {
            let target_normalized = link_target.trim_end_matches(".md").to_lowercase();
            if let Some(&target_ni) = lookup.get(&target_normalized) {
                if target_ni != *ni && !graph.graph.contains_edge(*ni, target_ni) {
                    graph.add_edge(*ni, target_ni, 1.0);
                }
            }
            // Also try with .md extension
            let with_ext = format!("{}.md", target_normalized);
            if let Some(&target_ni) = lookup.get(&with_ext) {
                if target_ni != *ni && !graph.graph.contains_edge(*ni, target_ni) {
                    graph.add_edge(*ni, target_ni, 1.0);
                }
            }
        }
    }

    // If no edges at all, connect isolated nodes in a weak chain
    // so the graph has SOME structure visible
    if graph.edge_count() == 0 && node_count >= 2 {
        let indices: Vec<NodeIndex> = graph.graph.node_indices().collect();
        for pair in indices.windows(2) {
            graph.add_edge(pair[0], pair[1], 0.3);
        }
    }
}
