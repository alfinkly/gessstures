use crate::hand_tracking::HandLandmark;

/// Recognized hand gestures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    OpenPalm,
    Fist,
    Pinch,
    Movement,
    Unknown,
}

/// Result produced by [`GestureClassifier::classify`] for a single frame.
#[derive(Debug, Clone)]
pub struct GestureResult {
    pub gesture: Gesture,
    pub confidence: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
}

/// Pure-data gesture classifier based on 21 hand landmarks.
///
/// Uses simple geometric heuristics (finger-tip distances) — no ML involved.
/// Maintains frame-to-frame state for smoothing and movement tracking.
pub struct GestureClassifier {
    pub prev_center_x: f32,
    pub prev_center_y: f32,
}

impl GestureClassifier {
    pub fn new() -> Self {
        Self {
            prev_center_x: 0.5,
            prev_center_y: 0.5,
        }
    }

    /// Classify a single frame of 21 hand landmarks.
    ///
    /// Returns a [`GestureResult`] with the detected gesture, confidence,
    /// smoothed cursor position, and frame-to-frame delta.
    ///
    /// If fewer than 21 landmarks are provided, returns [`Gesture::Unknown`]
    /// with zero confidence and no movement.
    pub fn classify(&mut self, landmarks: &[HandLandmark]) -> GestureResult {
        if landmarks.len() < 21 {
            return GestureResult {
                gesture: Gesture::Unknown,
                confidence: 0.0,
                cursor_x: self.prev_center_x,
                cursor_y: self.prev_center_y,
                delta_x: 0.0,
                delta_y: 0.0,
            };
        }

        let raw_center_x = (landmarks[5].x + landmarks[9].x + landmarks[13].x + landmarks[17].x) / 4.0;
        let raw_center_y = (landmarks[5].y + landmarks[9].y + landmarks[13].y + landmarks[17].y) / 4.0;

        let alpha = 0.3;
        let center_x = alpha * raw_center_x + (1.0 - alpha) * self.prev_center_x;
        let center_y = alpha * raw_center_y + (1.0 - alpha) * self.prev_center_y;

        let delta_x = center_x - self.prev_center_x;
        let delta_y = center_y - self.prev_center_y;

        self.prev_center_x = center_x;
        self.prev_center_y = center_y;

        let distances = [
            distance_3d(&landmarks[4], &landmarks[5]),
            distance_3d(&landmarks[8], &landmarks[9]),
            distance_3d(&landmarks[12], &landmarks[13]),
            distance_3d(&landmarks[16], &landmarks[17]),
            distance_3d(&landmarks[20], &landmarks[0]),
        ];

        let is_fist = distances.iter().all(|d| *d < 0.06);

        let is_open_palm = distances.iter().all(|d| *d > 0.1);

        let pinch_dist = distance_3d(&landmarks[4], &landmarks[8]);
        let is_pinch = pinch_dist < 0.05;

        if is_fist {
            let avg_dist = distances.iter().sum::<f32>() / distances.len() as f32;
            let confidence = 1.0 - (avg_dist / 0.06).min(1.0);
            return GestureResult {
                gesture: Gesture::Fist,
                confidence,
                cursor_x: center_x,
                cursor_y: center_y,
                delta_x,
                delta_y,
            };
        }

        if is_open_palm {
            let avg_dist = distances.iter().sum::<f32>() / distances.len() as f32;
            let confidence = ((avg_dist - 0.1) / 0.1).min(1.0).max(0.0);
            return GestureResult {
                gesture: Gesture::OpenPalm,
                confidence,
                cursor_x: center_x,
                cursor_y: center_y,
                delta_x,
                delta_y,
            };
        }

        if is_pinch {
            let confidence = 1.0 - (pinch_dist / 0.05).min(1.0);
            return GestureResult {
                gesture: Gesture::Pinch,
                confidence,
                cursor_x: center_x,
                cursor_y: center_y,
                delta_x,
                delta_y,
            };
        }

        let movement_mag = (delta_x.powi(2) + delta_y.powi(2)).sqrt();
        if movement_mag > 0.005 {
            let confidence = (movement_mag / 0.02).min(1.0);
            return GestureResult {
                gesture: Gesture::Movement,
                confidence,
                cursor_x: center_x,
                cursor_y: center_y,
                delta_x,
                delta_y,
            };
        }

        GestureResult {
            gesture: Gesture::Unknown,
            confidence: 0.0,
            cursor_x: center_x,
            cursor_y: center_y,
            delta_x,
            delta_y,
        }
    }
}

impl Default for GestureClassifier {
    fn default() -> Self {
        Self::new()
    }
}

fn distance_3d(a: &HandLandmark, b: &HandLandmark) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_landmarks(override_indices: &[(usize, f32, f32, f32)]) -> Vec<HandLandmark> {
        let mut lms: Vec<HandLandmark> = (0..21)
            .map(|_| HandLandmark {
                x: 0.5,
                y: 0.5,
                z: 0.0,
            })
            .collect();

        lms[0] = HandLandmark {
            x: 0.5,
            y: 0.8,
            z: 0.0,
        };

        for &(idx, x, y, z) in override_indices {
            if idx < 21 {
                lms[idx] = HandLandmark { x, y, z };
            }
        }
        lms
    }

    fn open_palm_landmarks() -> Vec<HandLandmark> {
        make_landmarks(&[
            (0, 0.5, 0.80, 0.0),
            (1, 0.70, 0.75, 0.0),
            (2, 0.75, 0.70, 0.0),
            (3, 0.78, 0.65, 0.0),
            (4, 0.82, 0.58, 0.0),
            (5, 0.58, 0.60, 0.0),
            (6, 0.58, 0.45, 0.0),
            (7, 0.58, 0.30, 0.0),
            (8, 0.58, 0.20, 0.0),
            (9, 0.50, 0.60, 0.0),
            (10, 0.50, 0.42, 0.0),
            (11, 0.50, 0.25, 0.0),
            (12, 0.50, 0.15, 0.0),
            (13, 0.42, 0.60, 0.0),
            (14, 0.42, 0.45, 0.0),
            (15, 0.42, 0.30, 0.0),
            (16, 0.42, 0.20, 0.0),
            (17, 0.35, 0.62, 0.0),
            (18, 0.35, 0.50, 0.0),
            (19, 0.35, 0.38, 0.0),
            (20, 0.35, 0.28, 0.0),
        ])
    }

    #[test]
    fn test_open_palm() {
        let landmarks = open_palm_landmarks();
        let mut classifier = GestureClassifier::new();
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::OpenPalm,
            "expected OpenPalm, got {:?} (confidence {})",
            result.gesture,
            result.confidence,
        );
        assert!(result.confidence > 0.0, "confidence should be > 0");
    }

    fn fist_landmarks() -> Vec<HandLandmark> {
        make_landmarks(&[
            (0, 0.45, 0.65, 0.0),
            (1, 0.54, 0.63, 0.0),
            (2, 0.55, 0.62, 0.0),
            (3, 0.54, 0.61, 0.0),
            (4, 0.53, 0.60, 0.0),
            (5, 0.52, 0.60, 0.0),
            (6, 0.51, 0.59, 0.0),
            (7, 0.50, 0.58, 0.0),
            (8, 0.49, 0.58, 0.0),
            (9, 0.48, 0.60, 0.0),
            (10, 0.47, 0.59, 0.0),
            (11, 0.46, 0.58, 0.0),
            (12, 0.46, 0.59, 0.0),
            (13, 0.44, 0.60, 0.0),
            (14, 0.44, 0.59, 0.0),
            (15, 0.43, 0.58, 0.0),
            (16, 0.43, 0.59, 0.0),
            (17, 0.40, 0.62, 0.0),
            (18, 0.41, 0.61, 0.0),
            (19, 0.42, 0.60, 0.0),
            (20, 0.42, 0.63, 0.0),
        ])
    }

    #[test]
    fn test_fist() {
        let landmarks = fist_landmarks();
        let mut classifier = GestureClassifier::new();
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::Fist,
            "expected Fist, got {:?} (confidence {})",
            result.gesture,
            result.confidence,
        );
        assert!(result.confidence > 0.0, "confidence should be > 0");
    }

    fn pinch_landmarks() -> Vec<HandLandmark> {
        make_landmarks(&[
            (0, 0.5, 0.80, 0.0),
            (1, 0.60, 0.72, 0.0),
            (2, 0.62, 0.68, 0.0),
            (3, 0.63, 0.65, 0.0),
            (4, 0.63, 0.62, 0.0),
            (5, 0.55, 0.60, 0.0),
            (6, 0.56, 0.55, 0.0),
            (7, 0.58, 0.52, 0.0),
            (8, 0.62, 0.61, 0.0),
            (9, 0.48, 0.60, 0.0),
            (10, 0.48, 0.48, 0.0),
            (11, 0.48, 0.36, 0.0),
            (12, 0.48, 0.28, 0.0),
            (13, 0.42, 0.60, 0.0),
            (14, 0.42, 0.48, 0.0),
            (15, 0.42, 0.36, 0.0),
            (16, 0.42, 0.28, 0.0),
            (17, 0.36, 0.63, 0.0),
            (18, 0.36, 0.52, 0.0),
            (19, 0.36, 0.44, 0.0),
            (20, 0.36, 0.38, 0.0),
        ])
    }

    #[test]
    fn test_pinch() {
        let landmarks = pinch_landmarks();
        let mut classifier = GestureClassifier::new();
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::Pinch,
            "expected Pinch, got {:?} (confidence {})",
            result.gesture,
            result.confidence,
        );
        assert!(result.confidence > 0.0, "confidence should be > 0");
    }

    #[test]
    fn test_movement() {
        let mut classifier = GestureClassifier::new();

        let frame1 = open_palm_landmarks();
        let r1 = classifier.classify(&frame1);
        assert!(
            r1.cursor_x >= 0.0 && r1.cursor_x <= 1.0,
            "cursor_x out of range"
        );

        let mut frame2 = open_palm_landmarks();
        for lm in &mut frame2 {
            lm.x += 0.3;
        }
        frame2[16] = HandLandmark {
            x: 0.65,
            y: 0.55,
            z: 0.0,
        };
        let r2 = classifier.classify(&frame2);
        assert_eq!(
            r2.gesture,
            Gesture::Movement,
            "expected Movement, got {:?}",
            r2.gesture,
        );
        assert!(r2.delta_x > 0.0, "delta_x should be positive");
        assert!(r2.confidence > 0.0, "confidence should be > 0");
    }

    #[test]
    fn test_unknown_when_too_few_landmarks() {
        let landmarks = vec![HandLandmark {
            x: 0.5,
            y: 0.5,
            z: 0.0,
        }];
        let mut classifier = GestureClassifier::new();
        let result = classifier.classify(&landmarks);
        assert_eq!(result.gesture, Gesture::Unknown);
        assert_eq!(result.confidence, 0.0);
        assert_eq!(result.delta_x, 0.0);
        assert_eq!(result.delta_y, 0.0);
    }

    #[test]
    fn test_exponential_smoothing() {
        let mut classifier = GestureClassifier::new();
        let lms = open_palm_landmarks();
        let r = classifier.classify(&lms);
        let expected_x = 0.3 * 0.4625 + 0.7 * 0.5;
        let expected_y = 0.3 * 0.605 + 0.7 * 0.5;
        assert!(
            (r.cursor_x - expected_x).abs() < 0.001,
            "smoothing x: expected {:.5}, got {:.5}",
            expected_x,
            r.cursor_x,
        );
        assert!(
            (r.cursor_y - expected_y).abs() < 0.001,
            "smoothing y: expected {:.5}, got {:.5}",
            expected_y,
            r.cursor_y,
        );
    }

    #[test]
    fn test_fist_takes_priority_over_pinch() {
        let landmarks = make_landmarks(&[
            (0, 0.45, 0.65, 0.0),
            (4, 0.52, 0.61, 0.0),
            (5, 0.52, 0.60, 0.0),
            (8, 0.50, 0.60, 0.0),
            (9, 0.48, 0.60, 0.0),
            (12, 0.47, 0.59, 0.0),
            (13, 0.44, 0.60, 0.0),
            (16, 0.44, 0.59, 0.0),
            (17, 0.40, 0.62, 0.0),
            (20, 0.42, 0.63, 0.0),
        ]);
        let mut classifier = GestureClassifier::new();
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::Fist,
            "Fist should take priority over Pinch when both conditions are met, got {:?}",
            result.gesture,
        );
    }
}
