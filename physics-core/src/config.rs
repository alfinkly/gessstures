#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub repulsion_strength: f32,
    pub spring_strength: f32,
    pub spring_rest: f32,
    pub center_strength: f32,
    pub damping: f32,
    pub max_speed: f32,
    pub min_distance: f32,
    pub boundary: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            repulsion_strength: 2.0,
            spring_strength: 0.3,
            spring_rest: 8.0,
            center_strength: 0.03,
            damping: 0.97,
            max_speed: 0.12,
            min_distance: 0.5,
            boundary: 30.0,
        }
    }
}
