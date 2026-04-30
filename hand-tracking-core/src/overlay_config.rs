#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Clone, Debug)]
pub struct HandOverlayConfig {
    pub enabled: bool,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for HandOverlayConfig {
    fn default() -> Self {
        Self { enabled: false, left: 0.0, top: 0.0, width: 0.0, height: 0.0 }
    }
}
