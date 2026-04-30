use async_trait::async_trait;
use graph_core::NewContent;
use walkdir::WalkDir;

use crate::traits::DataSource;

pub struct MarkdownDataSource {
    pub root_path: String,
    name: String,
}

impl MarkdownDataSource {
    pub fn new(root_path: String, name: String) -> Self {
        Self { root_path, name }
    }
}

#[async_trait]
impl DataSource for MarkdownDataSource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn load(&self) -> anyhow::Result<Vec<NewContent>> {
        let mut contents = Vec::new();
        let root = std::path::Path::new(&self.root_path);

        for entry in WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                let Ok(text) = std::fs::read_to_string(path) else {
                    continue;
                };
                let relative = path.strip_prefix(root).unwrap_or(path);
                let source = relative.display().to_string();
                let (title, links) = parse_note(&text, &source, path);

                contents.push(NewContent {
                    source,
                    raw_text: text,
                    title,
                    outgoing_links: links,
                });
            }
        }
        Ok(contents)
    }
}

fn parse_note(text: &str, source: &str, path: &std::path::Path) -> (String, Vec<String>) {
    use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};

    let parser = Parser::new(text);
    let mut title = String::new();
    let mut md_links: Vec<String> = Vec::new();
    let mut in_h1 = false;
    let mut title_found = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_h1 = true;
            }
            Event::Text(t) | Event::Code(t) if in_h1 => {
                title.push_str(&t);
                title_found = true;
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                in_h1 = false;
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let target = dest_url.to_string();
                if !target.starts_with("http") {
                    md_links.push(target);
                }
            }
            _ => {}
        }
    }

    if !title_found {
        title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(source)
            .to_string();
    }

    md_links.extend(extract_wikilinks(text));

    (title, md_links)
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
