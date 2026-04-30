use bevy::prelude::*;
use graph_core::GraphQuery;

pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputBuffer>()
            .add_systems(Update, handle_keyboard_input);
    }
}

#[derive(Resource, Default)]
pub struct InputBuffer {
    pub buffer: String,
}

fn handle_keyboard_input(
    mut input: ResMut<InputBuffer>,
    mut graph_query: EventWriter<GraphQuery>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Enter) {
        if !input.buffer.is_empty() {
            info!("Query submitted: {}", input.buffer);
            graph_query.send(GraphQuery {
                query: input.buffer.clone(),
            });
            input.buffer.clear();
        }
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        input.buffer.pop();
        return;
    }

    if keys.just_pressed(KeyCode::Space) {
        input.buffer.push(' ');
        return;
    }

    // Letters A-Z
    for (key, base_char) in [
        (KeyCode::KeyA, 'a'),
        (KeyCode::KeyB, 'b'),
        (KeyCode::KeyC, 'c'),
        (KeyCode::KeyD, 'd'),
        (KeyCode::KeyE, 'e'),
        (KeyCode::KeyF, 'f'),
        (KeyCode::KeyG, 'g'),
        (KeyCode::KeyH, 'h'),
        (KeyCode::KeyI, 'i'),
        (KeyCode::KeyJ, 'j'),
        (KeyCode::KeyK, 'k'),
        (KeyCode::KeyL, 'l'),
        (KeyCode::KeyM, 'm'),
        (KeyCode::KeyN, 'n'),
        (KeyCode::KeyO, 'o'),
        (KeyCode::KeyP, 'p'),
        (KeyCode::KeyQ, 'q'),
        (KeyCode::KeyR, 'r'),
        (KeyCode::KeyS, 's'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyU, 'u'),
        (KeyCode::KeyV, 'v'),
        (KeyCode::KeyW, 'w'),
        (KeyCode::KeyX, 'x'),
        (KeyCode::KeyY, 'y'),
        (KeyCode::KeyZ, 'z'),
    ] {
        if keys.just_pressed(key) {
            let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            input.buffer.push(if shift {
                base_char.to_ascii_uppercase()
            } else {
                base_char
            });
        }
    }

    // Digits
    for (key, ch) in [
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
    ] {
        if keys.just_pressed(key) {
            input.buffer.push(ch);
        }
    }

    // Punctuation
    for (key, ch) in [
        (KeyCode::Period, '.'),
        (KeyCode::Comma, ','),
        (KeyCode::Minus, '-'),
        (KeyCode::Equal, '='),
        (KeyCode::Slash, '/'),
        (KeyCode::Backslash, '\\'),
        (KeyCode::BracketLeft, '['),
        (KeyCode::BracketRight, ']'),
        (KeyCode::Quote, '\''),
    ] {
        if keys.just_pressed(key) {
            input.buffer.push(ch);
        }
    }
}
