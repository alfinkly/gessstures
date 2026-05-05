use std::sync::Arc;

use clap::Parser;
use engine::Engine;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;

mod server;
mod loader;
mod face_tracker;
mod db;

#[derive(Parser)]
#[command(name = "ws-bridge", about = "Gessstures WebSocket bridge")]
struct Cli {
    #[arg(short, long, default_value = ".")]
    notes: String,

    #[arg(short, long, default_value = "3030")]
    port: u16,

    #[arg(long, default_value = "postgres://gessstures:gessstures@localhost:5432/gessstures")]
    database_url: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cli.database_url)
        .await
        .expect("Failed to connect to PostgreSQL");
    db::run_migrations(&pool).await;

    let mut engine = Engine::new();
    let contents = loader::load_markdown_folder(&cli.notes);
    println!("Loaded {} markdown files", contents.len());
    engine.add_contents(contents);
    engine.rebuild_node_order();
    engine.build_edges();
    println!("Graph: {} nodes, {} edges", engine.graph.node_count(), engine.graph.edge_count());

    let engine = Arc::new(Mutex::new(engine));
    let tracker = Arc::new(Mutex::new(face_tracker::FaceTracker::new(pool).await));

    let engine_clone = engine.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs_f64(1.0 / 60.0));
        loop {
            interval.tick().await;
            let mut eng = engine_clone.lock().await;
            eng.tick();
        }
    });

    let tr = tracker.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut t = tr.lock().await;
            t.close_stale_visits().await;
        }
    });

    server::start(cli.port, engine, tracker).await;
}
