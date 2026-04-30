use crate::hand_landmark::HandLandmark;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    OpenPalm,
    Fist,
    Pinch,
    Point,
    VSIGN,
    Movement,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct GestureConfig {
    pub smoothing_alpha: f32,        // 0.0-1.0, default 0.3
    pub dead_zone_xy: f32,           // min pixel movement to register, default 0.005
    pub dead_zone_z: f32,            // min z movement, default 0.003
    pub point_extend_threshold: f32, // default 0.15
    pub finger_curl_threshold: f32,  // default 0.08
    pub swipe_threshold: f32,        // default 0.03
    pub hold_frames: u32,            // default 30 (0.5s at 60fps)
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            smoothing_alpha: 0.3,
            dead_zone_xy: 0.005,
            dead_zone_z: 0.003,
            point_extend_threshold: 0.15,
            finger_curl_threshold: 0.08,
            swipe_threshold: 0.03,
            hold_frames: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GestureResult {
    pub gesture: Gesture,
    pub confidence: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub delta_z: f32,
    pub hand_open_ratio: f32,
}

pub struct GestureClassifier {
    pub prev_center_x: f32,
    pub prev_center_y: f32,
    pub prev_avg_z: f32,
    pub config: GestureConfig,
}

impl GestureClassifier {
    pub fn new(config: GestureConfig) -> Self {
        Self {
            prev_center_x: 0.5,
            prev_center_y: 0.5,
            prev_avg_z: 0.0,
            config,
        }
    }

    pub fn classify(&mut self, landmarks: &[HandLandmark]) -> GestureResult {
        if landmarks.len() < 21 {
            return GestureResult {
                gesture: Gesture::Unknown,
                confidence: 0.0,
                cursor_x: self.prev_center_x,
                cursor_y: self.prev_center_y,
                delta_x: 0.0,
                delta_y: 0.0,
                delta_z: 0.0,
                hand_open_ratio: 0.0,
            };
        }

        let raw_center_x = (landmarks[5].x + landmarks[9].x + landmarks[13].x + landmarks[17].x) / 4.0;
        let raw_center_y = (landmarks[5].y + landmarks[9].y + landmarks[13].y + landmarks[17].y) / 4.0;
        let raw_avg_z = (landmarks[5].z + landmarks[9].z + landmarks[13].z + landmarks[17].z) / 4.0;

        let alpha = self.config.smoothing_alpha;
        let center_x = alpha * raw_center_x + (1.0 - alpha) * self.prev_center_x;
        let center_y = alpha * raw_center_y + (1.0 - alpha) * self.prev_center_y;

        let raw_delta_x = center_x - self.prev_center_x;
        let raw_delta_y = center_y - self.prev_center_y;
        let raw_delta_z = raw_avg_z - self.prev_avg_z;

        let delta_x = if raw_delta_x.abs() < self.config.dead_zone_xy { 0.0 } else { raw_delta_x };
        let delta_y = if raw_delta_y.abs() < self.config.dead_zone_xy { 0.0 } else { raw_delta_y };
        let delta_z = if raw_delta_z.abs() < self.config.dead_zone_z { 0.0 } else { raw_delta_z };

        self.prev_center_x = center_x;
        self.prev_center_y = center_y;
        self.prev_avg_z = raw_avg_z;

        let distances = [
            distance_3d(&landmarks[4], &landmarks[5]),
            distance_3d(&landmarks[8], &landmarks[9]),
            distance_3d(&landmarks[12], &landmarks[13]),
            distance_3d(&landmarks[16], &landmarks[17]),
            distance_3d(&landmarks[20], &landmarks[0]),
        ];

        let hand_open_ratio = distances
            .iter()
            .map(|d| ((d - 0.03) / 0.12).clamp(0.0, 1.0))
            .sum::<f32>()
            / distances.len() as f32;

        // --- VSIGN detection (highest priority among new gestures) ---
        {
            let idx_ext = distance_3d(&landmarks[8], &landmarks[5]);
            let mid_ext = distance_3d(&landmarks[12], &landmarks[9]);
            let ring_curl = distance_3d(&landmarks[16], &landmarks[13]);
            let pinky_curl = distance_3d(&landmarks[20], &landmarks[17]);

            if idx_ext > self.config.point_extend_threshold && mid_ext > self.config.point_extend_threshold && ring_curl < self.config.finger_curl_threshold && pinky_curl < self.config.finger_curl_threshold {
                let idx_conf = ((idx_ext - self.config.point_extend_threshold) / self.config.point_extend_threshold).clamp(0.0, 1.0);
                let mid_conf = ((mid_ext - self.config.point_extend_threshold) / self.config.point_extend_threshold).clamp(0.0, 1.0);
                let confidence = (idx_conf + mid_conf) / 2.0;
                return GestureResult {
                    gesture: Gesture::VSIGN,
                    confidence,
                    cursor_x: center_x,
                    cursor_y: center_y,
                    delta_x,
                    delta_y,
                    delta_z,
                    hand_open_ratio,
                };
            }
        }

        // --- Point detection ---
        {
            let idx_ext = distance_3d(&landmarks[8], &landmarks[5]);
            let mid_curl = distance_3d(&landmarks[12], &landmarks[9]);
            let ring_curl = distance_3d(&landmarks[16], &landmarks[13]);
            let pinky_curl = distance_3d(&landmarks[20], &landmarks[17]);
            let thumb_spread = distance_3d(&landmarks[4], &landmarks[5]);

            if idx_ext > self.config.point_extend_threshold
                && mid_curl < self.config.finger_curl_threshold
                && ring_curl < self.config.finger_curl_threshold
                && pinky_curl < self.config.finger_curl_threshold
                && thumb_spread > 0.08
            {
                let confidence = ((idx_ext - self.config.point_extend_threshold) / self.config.point_extend_threshold).clamp(0.0, 1.0);
                return GestureResult {
                    gesture: Gesture::Point,
                    confidence,
                    cursor_x: center_x,
                    cursor_y: center_y,
                    delta_x,
                    delta_y,
                    delta_z,
                    hand_open_ratio,
                };
            }
        }

        // --- Fist detection ---
        let is_fist = distances.iter().all(|d| *d < 0.06);

        // --- Open palm detection ---
        let is_open_palm = distances.iter().all(|d| *d > 0.1);

        // --- Pinch detection ---
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
                delta_z,
                hand_open_ratio,
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
                delta_z,
                hand_open_ratio,
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
                delta_z,
                hand_open_ratio,
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
                delta_z,
                hand_open_ratio,
            };
        }

        GestureResult {
            gesture: Gesture::Unknown,
            confidence: 0.0,
            cursor_x: center_x,
            cursor_y: center_y,
            delta_x,
            delta_y,
            delta_z,
            hand_open_ratio,
        }
    }
}

impl Default for GestureClassifier {
    fn default() -> Self {
        Self::new(GestureConfig::default())
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
        let mut classifier = GestureClassifier::new(GestureConfig::default());
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
        let mut classifier = GestureClassifier::new(GestureConfig::default());
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
        let mut classifier = GestureClassifier::new(GestureConfig::default());
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
        let mut classifier = GestureClassifier::new(GestureConfig::default());

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
        let mut classifier = GestureClassifier::new(GestureConfig::default());
        let result = classifier.classify(&landmarks);
        assert_eq!(result.gesture, Gesture::Unknown);
        assert_eq!(result.confidence, 0.0);
        assert_eq!(result.delta_x, 0.0);
        assert_eq!(result.delta_y, 0.0);
    }

    #[test]
    fn test_exponential_smoothing() {
        let mut classifier = GestureClassifier::new(GestureConfig::default());
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
        let mut classifier = GestureClassifier::new(GestureConfig::default());
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::Fist,
            "Fist should take priority over Pinch when both conditions are met, got {:?}",
            result.gesture,
        );
    }

    fn point_landmarks() -> Vec<HandLandmark> {
        make_landmarks(&[
            (4, 0.62, 0.50, 0.0),
            (8, 0.50, 0.20, 0.0),
            (12, 0.50, 0.48, 0.0),
            (16, 0.50, 0.48, 0.0),
            (20, 0.50, 0.48, 0.0),
        ])
    }

    #[test]
    fn test_point() {
        let landmarks = point_landmarks();
        let mut classifier = GestureClassifier::new(GestureConfig::default());
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::Point,
            "expected Point, got {:?} (confidence {})",
            result.gesture,
            result.confidence,
        );
        assert!(result.confidence > 0.0, "confidence should be > 0");
    }

    fn vsign_landmarks() -> Vec<HandLandmark> {
        make_landmarks(&[
            (8, 0.50, 0.20, 0.0),
            (12, 0.50, 0.20, 0.0),
            (16, 0.50, 0.48, 0.0),
            (20, 0.50, 0.48, 0.0),
        ])
    }

    #[test]
    fn test_vsign() {
        let landmarks = vsign_landmarks();
        let mut classifier = GestureClassifier::new(GestureConfig::default());
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::VSIGN,
            "expected VSIGN, got {:?} (confidence {})",
            result.gesture,
            result.confidence,
        );
        assert!(result.confidence > 0.0, "confidence should be > 0");
    }

    #[test]
    fn test_point_before_fist() {
        let landmarks = make_landmarks(&[
            (4, 0.62, 0.50, 0.0),
            (8, 0.50, 0.20, 0.0),
            (12, 0.50, 0.48, 0.0),
            (16, 0.50, 0.48, 0.0),
            (20, 0.50, 0.48, 0.0),
        ]);
        let mut classifier = GestureClassifier::new(GestureConfig::default());
        let result = classifier.classify(&landmarks);
        assert_eq!(
            result.gesture,
            Gesture::Point,
            "Point should take priority over Fist when index is extended, got {:?}",
            result.gesture,
        );
    }

    #[test]
    fn test_dead_zone() {
        let mut classifier = GestureClassifier::new(GestureConfig::default());

        let frame1 = make_landmarks(&[]);
        classifier.classify(&frame1);

        let mut frame2 = make_landmarks(&[]);
        for lm in &mut frame2 {
            lm.x += 0.001;
        }
        let result = classifier.classify(&frame2);
        assert_eq!(result.delta_x, 0.0, "delta_x should be zero (dead zone)");
        assert_eq!(result.delta_y, 0.0, "delta_y should be zero (dead zone)");
    }
}
