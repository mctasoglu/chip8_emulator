use crate::cpu::{Chip8Cpu, FONT_SET};
use minifb::{MouseButton, MouseMode, Window};

// Keypad region: starts at x=600 and fills the remaining 400px to the right
// of the window (whose total width/height are defined in main.rs).
pub const KEYPAD_X_START: usize = 600;
// Size of one button: 400px / 4 columns = 100px wide, 500px / 4 rows = 125px tall.
pub const KEY_WIDTH: usize = (crate::WIDTH - KEYPAD_X_START) / 4;
pub const KEY_HEIGHT: usize = crate::HEIGHT / 4;
// Gap (in pixels) inset from each button's edge so buttons don't touch.
pub const KEY_MARGIN: usize = 4;
// How big each font-sprite pixel gets drawn as, for the key labels.
pub const LABEL_SCALE: usize = 12;

// Real CHIP-8 keypad layout (not numeric order!) mapped to grid position:
// KEYPAD_LAYOUT[row][col] gives the hex key value drawn/checked at that cell.
//   1 2 3 C
//   4 5 6 D
//   7 8 9 E
//   A 0 B F
pub const KEYPAD_LAYOUT: [[u8; 4]; 4] = [
    [0x1, 0x2, 0x3, 0xC],
    [0x4, 0x5, 0x6, 0xD],
    [0x7, 0x8, 0x9, 0xE],
    [0xA, 0x0, 0xB, 0xF],
];

// Clear all keys, then hit-test the mouse against the keypad grid and mark
// the key under the cursor as pressed, if the left button is held.
pub fn update(cpu: &mut Chip8Cpu, window: &Window) {
    for key in cpu.keypad.iter_mut() {
        *key = false;
    }

    if window.get_mouse_down(MouseButton::Left) {
        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Discard) {
            let mx = mx as usize;
            let my = my as usize;
            if mx >= KEYPAD_X_START {
                let col = (mx - KEYPAD_X_START) / KEY_WIDTH;
                let row = my / KEY_HEIGHT;
                if row < 4 && col < 4 {
                    let key = KEYPAD_LAYOUT[row][col];
                    cpu.keypad[key as usize] = true;
                }
            }
        }
    }
}

// Draw all 16 keypad buttons (green while held, dark gray otherwise), each
// labeled with its hex digit using the CHIP-8 font glyphs.
pub fn draw(cpu: &Chip8Cpu, buffer: &mut [u32], width: usize) {
    for row in 0..4 {
        for col in 0..4 {
            let key = KEYPAD_LAYOUT[row][col];
            let color = if cpu.keypad[key as usize] {
                0x00FF00
            } else {
                0x444444
            };

            // This cell's pixel rectangle, inset by KEY_MARGIN on each side.
            let x_start = KEYPAD_X_START + col * KEY_WIDTH + KEY_MARGIN;
            let x_end = KEYPAD_X_START + (col + 1) * KEY_WIDTH - KEY_MARGIN;
            let y_start = row * KEY_HEIGHT + KEY_MARGIN;
            let y_end = (row + 1) * KEY_HEIGHT - KEY_MARGIN;

            // Paint every pixel in the rectangle. `buffer` is a flat 1D
            // array representing a 2D image, so pixel (x, y) lives at
            // index y * width + x.
            for y in y_start..y_end {
                for x in x_start..x_end {
                    buffer[y * width + x] = color;
                }
            }

            // Draw the key's label on top of the button background,
            // using the CHIP-8 font glyph for this hex digit, centered
            // in the button's interior.
            let label_color = if cpu.keypad[key as usize] {
                0x000000
            } else {
                0xFFFFFF
            };
            let glyph = FONT_SET[key as usize];
            let label_width = 4 * LABEL_SCALE;
            let label_height = 5 * LABEL_SCALE;
            let label_x = x_start + (x_end - x_start - label_width) / 2;
            let label_y = y_start + (y_end - y_start - label_height) / 2;

            for (gy, byte) in glyph.iter().enumerate() {
                for gx in 0..4 {
                    let bit = (byte >> (7 - gx)) & 1;
                    if bit == 1 {
                        let px = label_x + gx * LABEL_SCALE;
                        let py = label_y + gy * LABEL_SCALE;
                        for y in py..py + LABEL_SCALE {
                            for x in px..px + LABEL_SCALE {
                                buffer[y * width + x] = label_color;
                            }
                        }
                    }
                }
            }
        }
    }
}
