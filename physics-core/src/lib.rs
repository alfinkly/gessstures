//! Pure force-directed graph physics engine.
//!
//! Framework-agnostic: takes positions and edges, returns new positions.
//! Zero dependencies — no Bevy, no glam. Just math.

mod vec3;
mod config;
mod engine;

pub use vec3::Vec3;
pub use config::PhysicsConfig;
pub use engine::{PhysicsState, tick_physics};

pub type NodePositions = Vec<Vec3>;
pub type NodeVelocities = Vec<Vec3>;
pub type EdgeRef = (usize, usize);
