use bevy::prelude::*;

use crate::infrastructure::body::person_tracker::{PersonData, PersonTrackerResource};
use crate::interface::plugins::body_overlay_plugin::PersonTile;

/// Standard MediaPipe Pose topology: 33 landmarks, 39 connections.
/// Groups are color-coded for visual clarity.
const POSE_CONNECTIONS: &[(usize, usize, PoseGroup)] = &[
    // Face (YELLOW)
    (0, 1, PoseGroup::Face),
    (1, 2, PoseGroup::Face),
    (2, 3, PoseGroup::Face),
    (3, 7, PoseGroup::Face),
    (0, 4, PoseGroup::Face),
    (4, 5, PoseGroup::Face),
    (5, 6, PoseGroup::Face),
    (6, 8, PoseGroup::Face),
    (9, 10, PoseGroup::Face),
    // Shoulders (included in Torso group)
    (11, 12, PoseGroup::Torso),
    (11, 23, PoseGroup::Torso),
    (12, 24, PoseGroup::Torso),
    // L arm (GREEN)
    (11, 13, PoseGroup::LArm),
    (13, 15, PoseGroup::LArm),
    (15, 17, PoseGroup::LArm),
    (15, 19, PoseGroup::LArm),
    (15, 21, PoseGroup::LArm),
    (17, 19, PoseGroup::LArm),
    // R arm (CYAN)
    (12, 14, PoseGroup::RArm),
    (14, 16, PoseGroup::RArm),
    (16, 18, PoseGroup::RArm),
    (16, 20, PoseGroup::RArm),
    (16, 22, PoseGroup::RArm),
    (18, 20, PoseGroup::RArm),
    // Torso (ORANGE)
    (23, 24, PoseGroup::Torso),
    (23, 25, PoseGroup::Torso),
    (25, 27, PoseGroup::Torso),
    (24, 26, PoseGroup::Torso),
    (26, 28, PoseGroup::Torso),
    // Legs (PURPLE)
    (27, 29, PoseGroup::Legs),
    (29, 31, PoseGroup::Legs),
    (27, 31, PoseGroup::Legs),
    (28, 30, PoseGroup::Legs),
    (30, 32, PoseGroup::Legs),
    (28, 32, PoseGroup::Legs),
];

#[derive(Clone, Copy)]
enum PoseGroup {
    Face,
    LArm,
    RArm,
    Torso,
    Legs,
}

fn group_color(group: PoseGroup) -> Color {
    match group {
        PoseGroup::Face => Color::srgb(1.0, 1.0, 0.0),    // YELLOW
        PoseGroup::LArm => Color::srgb(0.0, 1.0, 0.0),    // GREEN
        PoseGroup::RArm => Color::srgb(0.0, 1.0, 1.0),    // CYAN
        PoseGroup::Torso => Color::srgb(1.0, 0.5, 0.0),   // ORANGE
        PoseGroup::Legs => Color::srgb(0.5, 0.0, 0.5),    // PURPLE
    }
}

pub struct PoseSkeletonPlugin;

impl Plugin for PoseSkeletonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_pose_skeletons);
    }
}

fn draw_pose_skeletons(
    person_tracker: Res<PersonTrackerResource>,
    mut gizmos: Gizmos,
    query: Query<(&PersonTile, &Sprite)>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.get_single() else { return };
    let ww = window.width();
    let wh = window.height();
    if ww == 0.0 || wh == 0.0 {
        return;
    }

    for (tile, _sprite) in &query {
        let idx = tile.index;
        if idx >= person_tracker.persons.len() {
            continue;
        }
        let person: &PersonData = &person_tracker.persons[idx];

        let Some(keypoints) = &person.keypoints else { continue };
        if keypoints.len() < 33 {
            continue;
        }

        let bbox = &person.bbox;

        // Bounding box edges in normalized [0,1] coordinates
        let bbox_left = bbox.cx - bbox.w / 2.0;
        let bbox_top = bbox.cy - bbox.h / 2.0;

        let screen_rect = tile.screen_rect;

        // Map a keypoint (x, y, visibility) to world-space gizmo position.
        // rel_x, rel_y are relative position within the bbox → mapped to tile screen coords → world coords.
        let kp_to_world = |kp: &[f32; 3]| -> Vec2 {
            let rel_x = (kp[0] - bbox_left) / bbox.w;
            let rel_y = (kp[1] - bbox_top) / bbox.h;
            // Screen coords (Y-down, origin top-left)
            let sx = screen_rect.min.x + rel_x * screen_rect.width();
            let sy = screen_rect.min.y + (1.0 - rel_y) * screen_rect.height();
            // Convert to Bevy world coords (Y-up, origin center)
            Vec2::new(sx - ww / 2.0, wh / 2.0 - sy)
        };

        // Draw connections
        for &(a, b, group) in POSE_CONNECTIONS {
            if a >= keypoints.len() || b >= keypoints.len() {
                continue;
            }
            let pa = kp_to_world(&keypoints[a]);
            let pb = kp_to_world(&keypoints[b]);
            gizmos.line_2d(pa, pb, group_color(group));
        }

        // Draw landmarks as small circles
        for kp in keypoints.iter().take(33) {
            let pos = kp_to_world(kp);
            gizmos.circle_2d(pos, 3.0, Color::WHITE);
        }
    }
}
