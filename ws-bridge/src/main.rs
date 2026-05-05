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

    #[arg(long)]
    database_url: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // optional PostgreSQL — without it, runs in-memory
    let tracker = if let Some(ref url) = cli.database_url {
        println!("[main] connecting to PostgreSQL...");
        match PgPoolOptions::new().max_connections(5).connect(url).await {
            Ok(pool) => {
                db::run_migrations(&pool).await;
                let tr = face_tracker::FaceTracker::new(pool).await;
                let tr = Arc::new(Mutex::new(tr));

                let stale_tr = tr.clone();
                tokio::spawn(async move {
                    let mut int = tokio::time::interval(std::time::Duration::from_secs(1));
                    loop {
                        int.tick().await;
                        let mut t = stale_tr.lock().await;
                        t.close_stale_visits().await;
                    }
                });

                tr
            }
            Err(e) => {
                eprintln!("[main] PostgreSQL unavailable ({}), falling back to in-memory", e);
                let tr = face_tracker::FaceTracker::new_memory();
                Arc::new(Mutex::new(tr))
            }
        }
    } else {
        println!("[main] no --database-url, running in-memory");
        let tr = face_tracker::FaceTracker::new_memory();
        Arc::new(Mutex::new(tr))
    };

    let mut engine = Engine::new();
    let contents = loader::load_markdown_folder(&cli.notes);
    println!("Loaded {} markdown files", contents.len());
    engine.add_contents(contents);
    engine.rebuild_node_order();
    engine.build_edges();
    println!("Graph: {} nodes, {} edges", engine.graph.node_count(), engine.graph.edge_count());

    let engine = Arc::new(Mutex::new(engine));

    let engine_clone = engine.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs_f64(1.0 / 60.0));
        loop {
            interval.tick().await;
            let mut eng = engine_clone.lock().await;
            eng.tick();
        }
    });

    server::start(cli.port, engine, tracker).await;
}
