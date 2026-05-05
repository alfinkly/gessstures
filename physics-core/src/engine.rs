use crate::{Vec3, PhysicsConfig, EdgeRef};

#[derive(Debug, Clone)]
pub struct PhysicsState {
    pub positions: Vec<Vec3>,
    pub velocities: Vec<Vec3>,
}

impl PhysicsState {
    pub fn new(node_count: usize) -> Self {
        Self {
            positions: vec![Vec3::ZERO; node_count],
            velocities: vec![Vec3::ZERO; node_count],
        }
    }
}

pub fn tick_physics(
    positions: &[Vec3],
    edges: &[EdgeRef],
    velocities: &[Vec3],
    config: &PhysicsConfig,
    paused: bool,
) -> (Vec<Vec3>, Vec<Vec3>) {
    let n = positions.len();
    if n < 2 {
        return (positions.to_vec(), vec![Vec3::ZERO; n]);
    }

    let mut new_velocities = velocities.to_vec();
    if new_velocities.len() != n {
        new_velocities = vec![Vec3::ZERO; n];
    }

    if paused {
        return (positions.to_vec(), new_velocities);
    }

    for i in 0..n {
        let pos_i = positions[i];

        let dist_center = pos_i.length();
        if dist_center > 0.01 {
            new_velocities[i] = new_velocities[i] + (pos_i * -1.0).normalize() * dist_center * config.center_strength;
        }

        if dist_center > config.boundary {
            new_velocities[i] = new_velocities[i] + (pos_i * -1.0).normalize() * (dist_center - config.boundary) * 1.5;
        }

        for j in 0..n {
            if i == j {
                continue;
            }
            let pos_j = positions[j];
            let dir = pos_i - pos_j;
            let dist = dir.length().max(config.min_distance);
            new_velocities[i] = new_velocities[i] + dir.normalize_or_zero() * config.repulsion_strength / (dist * dist);
        }

        for &(from, to) in edges {
            let (_target, neighbor_pos) = if from == i {
                if to >= n { continue; }
                (to, positions[to])
            } else if to == i {
                (from, positions[from])
            } else {
                continue;
            };

            let dir = neighbor_pos - pos_i;
            let dist = dir.length().max(config.min_distance);
            let force = config.spring_strength * (dist - config.spring_rest);
            new_velocities[i] = new_velocities[i] + dir.normalize_or_zero() * force;
        }
    }

    for v in &mut new_velocities {
        *v = *v * config.damping;
        let speed = v.length();
        if speed > config.max_speed {
            *v = v.normalize() * config.max_speed;
        }
    }

    let mut new_positions = positions.to_vec();
    for i in 0..n {
        new_positions[i] = new_positions[i] + new_velocities[i];
    }

    (new_positions, new_velocities)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_returns_unchanged() {
        let config = PhysicsConfig::default();
        let (pos, vel) = tick_physics(&[], &[], &[], &config, false);
        assert!(pos.is_empty());
        assert!(vel.is_empty());
    }

    #[test]
    fn single_node_stays() {
        let config = PhysicsConfig::default();
        let positions = vec![Vec3::new(1.0, 0.0, 0.0)];
        let (pos, _vel) = tick_physics(&positions, &[], &[Vec3::ZERO], &config, false);
        assert!((pos[0].x - 1.0).abs() < 0.01);
    }

    #[test]
    fn two_connected_nodes_attract() {
        let config = PhysicsConfig {
            spring_rest: 8.0,
            spring_strength: 1.0,
            damping: 1.0,
            repulsion_strength: 0.0,
            center_strength: 0.0,
            boundary: 100.0,
            ..Default::default()
        };
        let positions = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(20.0, 0.0, 0.0)];
        let edges = vec![(0, 1)];
        let (pos, _) = tick_physics(&positions, &edges, &[Vec3::ZERO; 2], &config, false);
        let new_dist = (pos[0] - pos[1]).length();
        assert!(new_dist < 20.0, "nodes should have moved closer together");
    }

    #[test]
    fn two_nodes_repel_without_edge() {
        let config = PhysicsConfig {
            repulsion_strength: 10.0,
            damping: 1.0,
            spring_strength: 0.0,
            center_strength: 0.0,
            boundary: 100.0,
            ..Default::default()
        };
        let positions = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
        let (pos, _) = tick_physics(&positions, &[], &[Vec3::ZERO; 2], &config, false);
        let new_dist = (pos[0] - pos[1]).length();
        assert!(new_dist > 1.0, "nodes should have moved apart: {new_dist}");
    }

    #[test]
    fn paused_returns_unchanged() {
        let config = PhysicsConfig::default();
        let positions = vec![Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)];
        let edges = vec![(0, 1)];
        let velocities = vec![Vec3::new(0.1, 0.0, 0.0); 2];
        let (pos, vel) = tick_physics(&positions, &edges, &velocities, &config, true);
        assert_eq!(pos, positions);
        assert!((vel[0].x - 0.1).abs() < 0.001);
    }
}
