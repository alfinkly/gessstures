use std::path::Path;
use graph_core::NewContent;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};

pub fn load_markdown_folder(path: &str) -> Vec<NewContent> {
    use walkdir::WalkDir;

    let root = Path::new(path);
    if !root.exists() {
        eprintln!("Notes folder '{}' does not exist", path);
        return vec![];
    }

    let mut contents = Vec::new();
    for entry in WalkDir::new(root).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        let file_path = entry.path();
        if file_path.extension().map_or(false, |ext| ext == "md") {
            let Ok(text) = std::fs::read_to_string(file_path) else {
                continue;
            };

            let relative = file_path.strip_prefix(root).unwrap_or(file_path);
            let source = relative.display().to_string();

            let (title, md_links) = parse_markdown(&text, file_path, &source);
            let wikilinks = extract_wikilinks(&text);
            let mut all_links = md_links;
            all_links.extend(wikilinks);

            contents.push(NewContent {
                source,
                raw_text: text,
                title,
                outgoing_links: all_links,
            });
        }
    }
    contents
}

fn parse_markdown(text: &str, file_path: &Path, source: &str) -> (String, Vec<String>) {
    let parser = Parser::new(text);
    let mut title = String::new();
    let mut md_links: Vec<String> = Vec::new();
    let mut in_h1 = false;
    let mut in_code_block = false;
    let mut title_found = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) if !in_code_block => {
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
                if !target.starts_with("http") && !target.starts_with('#') && target.contains('.') {
                    md_links.push(target);
                }
            }
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            _ => {}
        }
    }

    if !title_found {
        title = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(source)
            .to_string();
    }

    title.retain(|c| c != '<' && c != '>' && c != '|' && c != '=');
    if title.chars().count() > 80 {
        title = title.chars().take(80).collect();
    }
    title = title.trim().to_string();

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
