use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use std::env;
use std::fs;

// Total window size: left side is the game display, right side is the keypad.
const WIDTH: usize = 1000;
const HEIGHT: usize = 500;

// CHIP-8's native display resolution, and how many real pixels each CHIP-8
// pixel gets scaled up to when drawn into the window's game area.
const GFX_WIDTH: usize = 64;
const GFX_HEIGHT: usize = 32;
const GFX_SCALE: usize = 9;

// Keypad region: starts at x=600 and fills the remaining 400px to the right.
const KEYPAD_X_START: usize = 600;
// Size of one button: 400px / 4 columns = 100px wide, 500px / 4 rows = 125px tall.
const KEY_WIDTH: usize = (WIDTH - KEYPAD_X_START) / 4;
const KEY_HEIGHT: usize = HEIGHT / 4;
// Gap (in pixels) inset from each button's edge so buttons don't touch.
const KEY_MARGIN: usize = 4;
// How many fetch-execute cycles to run per rendered frame. At 60fps this is
// roughly a 600Hz CPU clock - runs the program continuously while still
// refreshing the display every frame, rather than all at once up front.
const CYCLES_PER_FRAME: usize = 10;
// How big each font-sprite pixel gets drawn as, for the key labels.
const LABEL_SCALE: usize = 12;

// The standard CHIP-8 built-in font set: one 4x5-pixel sprite per hex digit
// (0-F), same format DXYN draws (each byte is one row, top 4 bits are the
// pixels, read MSB-first). This is the same data a real interpreter loads
// into memory for the FX29 instruction later - we're just reusing it here to
// draw readable labels on the keypad buttons.
const FONT_SET: [[u8; 5]; 16] = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0], // 0
    [0x20, 0x60, 0x20, 0x20, 0x70], // 1
    [0xF0, 0x10, 0xF0, 0x80, 0xF0], // 2
    [0xF0, 0x10, 0xF0, 0x10, 0xF0], // 3
    [0x90, 0x90, 0xF0, 0x10, 0x10], // 4
    [0xF0, 0x80, 0xF0, 0x10, 0xF0], // 5
    [0xF0, 0x80, 0xF0, 0x90, 0xF0], // 6
    [0xF0, 0x10, 0x20, 0x40, 0x40], // 7
    [0xF0, 0x90, 0xF0, 0x90, 0xF0], // 8
    [0xF0, 0x90, 0xF0, 0x10, 0xF0], // 9
    [0xF0, 0x90, 0xF0, 0x90, 0x90], // A
    [0xE0, 0x90, 0xE0, 0x90, 0xE0], // B
    [0xF0, 0x80, 0x80, 0x80, 0xF0], // C
    [0xE0, 0x90, 0x90, 0x90, 0xE0], // D
    [0xF0, 0x80, 0xF0, 0x80, 0xF0], // E
    [0xF0, 0x80, 0xF0, 0x80, 0x80], // F
];

// Real CHIP-8 keypad layout (not numeric order!) mapped to grid position:
// KEYPAD_LAYOUT[row][col] gives the hex key value drawn/checked at that cell.
//   1 2 3 C
//   4 5 6 D
//   7 8 9 E
//   A 0 B F
const KEYPAD_LAYOUT: [[u8; 4]; 4] = [
    [0x1, 0x2, 0x3, 0xC],
    [0x4, 0x5, 0x6, 0xD],
    [0x7, 0x8, 0x9, 0xE],
    [0xA, 0x0, 0xB, 0xF],
];

// The CHIP-8 virtual machine's state: everything an interpreter needs to
// fetch, decode, and execute instructions.
struct Chip8Cpu {
    memory: [u8; 4096],       // 4KB of addressable memory (program + data).
    registers: [u8; 16],      // General-purpose registers V0-VF.
    i: u16,                   // Index register: holds an address (e.g. for sprite data).
    pc: u16,                  // Program counter: address of the next instruction to fetch.
    gfx: [u8; 64 * 32],       // Display buffer, one byte per pixel (64x32 monochrome).
    delay_timer: u8,          // Counts down at 60Hz; used for game timing.
    sound_timer: u8,          // Counts down at 60Hz; beeps while nonzero.
    stack: [u16; 16],         // Return addresses for CALL/RET (16 levels deep).
    stack_pointer: u8,        // Index of the next free slot in `stack`.
    keypad: [bool; 16],       // Which of the 16 hex keys (0x0-0xF) are currently held.
}

impl Chip8Cpu {
    // 00E0: clear the display.
    fn cls(&mut self) {
        self.gfx = [0; 64 * 32];
    }

    // 1NNN: jump to address NNN.
    fn jp(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    // 6XNN: set VX = NN.
    fn ld_vx_byte(&mut self, x: usize, nn: u8) {
        self.registers[x] = nn;
    }

    // 7XNN: VX += NN. CHIP-8 defines this as wrapping on overflow (no carry
    // flag set), unlike 8XY4. Plain `+` would panic on overflow in debug
    // builds, so we need `wrapping_add` to get the "just wrap around" behavior.
    fn add_vx_byte(&mut self, x: usize, nn: u8) {
        self.registers[x] = self.registers[x].wrapping_add(nn);
    }

    // ANNN: set I = NNN.
    fn ld_i(&mut self, nnn: u16) {
        self.i = nnn;
    }

    // Read the 2-byte instruction at `pc`, advance `pc` past it, and return
    // the combined opcode. Advancing *before* returning (rather than after)
    // is what lets a jump instruction (which assigns directly to `self.pc`)
    // simply overwrite this advance instead of it being undone or reapplied.
    fn fetch(&mut self) -> u16 {
        let hi = self.memory[self.pc as usize] as u16;
        let lo = self.memory[self.pc as usize + 1] as u16;
        self.pc += 2;
        (hi << 8) | lo
    }

    // DXYN: draw an N-row sprite (read from memory starting at I) at (VX, VY).
    // Each sprite row is a byte; each bit is one pixel, drawn via XOR.
    // VF is set to 1 if any pixel was flipped from on to off (a "collision").
    fn drw(&mut self, x: usize, y: usize, n: u8) {
        let vx = self.registers[x] as usize;
        let vy = self.registers[y] as usize;
        self.registers[0xF] = 0;

        for row in 0..n as usize {
            let sprite_byte = self.memory[self.i as usize + row];
            for col in 0..8 {
                let bit = (sprite_byte >> (7 - col)) & 1;
                if bit == 1 {
                    // Wrap around screen edges (64 wide, 32 tall).
                    let px = (vx + col) % 64;
                    let py = (vy + row) % 32;
                    let idx = py * 64 + px;

                    if self.gfx[idx] == 1 {
                        self.registers[0xF] = 1;
                    }
                    self.gfx[idx] ^= 1;
                }
            }
        }
    }
}

// Decode one opcode into its fields, then dispatch to the matching
// instruction method. Unrecognized/unimplemented opcodes just get logged.
fn handle_instruction(cpu: &mut Chip8Cpu, opcode: u16) {
    let x = ((opcode & 0x0F00) >> 8) as usize;
    let y = ((opcode & 0x00F0) >> 4) as usize;
    let n = (opcode & 0x000F) as u8;
    let nn = (opcode & 0x00FF) as u8;
    let nnn = opcode & 0x0FFF;

    match (opcode & 0xF000) >> 12 {
        0x0 => match opcode {
            0x00E0 => cpu.cls(),
            _ => println!("Unimplemented opcode: {:04X}", opcode),
        },
        0x1 => cpu.jp(nnn),
        0x6 => cpu.ld_vx_byte(x, nn),
        0x7 => cpu.add_vx_byte(x, nn),
        0xA => cpu.ld_i(nnn),
        0xD => cpu.drw(x, y, n),
        _ => println!("Unimplemented opcode: {:04X}", opcode),
    }
}

fn main() {
    // Pixel buffer that gets pushed to the window every frame (one u32 per pixel, 0xRRGGBB).
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    // ROM path comes from the first command-line argument, e.g.
    // `cargo run -- path/to/rom.ch8`.
    let rom_path = env::args()
        .nth(1)
        .expect("Usage: hello-rust <path-to-rom.ch8>");

    // Read the ROM file into memory as raw bytes.
    let rom_bytes: Vec<u8> = fs::read(&rom_path).expect("Unable to read file");
    println!("Read {} bytes from {}", rom_bytes.len(), rom_path);
    println!("First 10 bytes: {:?}", &rom_bytes[..10]);

    // Initial CPU state. `pc` starts at 0x200 since that's where CHIP-8
    // programs are conventionally loaded. (Still not implemented: a proper
    // `new()` constructor instead of this literal.)
    let mut cpu = Chip8Cpu {
        memory: [0; 4096],
        registers: [0; 16],
        i: 0,
        pc: 0x200,
        gfx: [0; 64 * 32],
        delay_timer: 0,
        sound_timer: 0,
        stack: [0; 16],
        stack_pointer: 0,
        keypad: [false; 16],
    };

    // Copy the ROM into memory starting at 0x200, so addresses baked into the
    // ROM's own opcodes (like ANNN's sprite pointer) line up with where the
    // bytes actually live in `cpu.memory`.
    cpu.memory[0x200..0x200 + rom_bytes.len()].copy_from_slice(&rom_bytes);

    // Disassembly dump only (no execution here): combine each 2-byte pair
    // (big-endian) into one opcode and print it. This just shows the raw
    // file contents in order - it does NOT reflect what actually runs, since
    // that's now driven by `cpu.pc` and jumps/loops below, not file order.
    for pair in rom_bytes.chunks(2) {
        if pair.len() == 2 {
            let opcode = (pair[0] as u16) << 8 | (pair[1] as u16);
            println!("Opcode: {:04X}", opcode);
        }
    }

    let mut window = Window::new(
        "Chip-8 Emulator - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Run a handful of real CPU cycles this frame: fetch the instruction
        // at `cpu.pc`, execute it (which may itself change `cpu.pc`, e.g.
        // JP), repeat. This is the actual PC-driven fetch-execute loop -
        // jumps and loops in the ROM now work correctly, forever, instead of
        // the one-shot straight-line walk we had before.
        for _ in 0..CYCLES_PER_FRAME {
            let opcode = cpu.fetch();
            handle_instruction(&mut cpu, opcode);
        }

        // Placeholder: fill the whole buffer with one flat color (game display
        // isn't wired up to `cpu.gfx` yet). Gets partially overwritten below
        // by the keypad drawing.
        for i in buffer.iter_mut() {
            *i = 277; // write something more funny here!
        }

        // Render cpu.gfx (the actual CHIP-8 display, 64x32) into the game
        // area, scaling each CHIP-8 pixel up to a GFX_SCALE x GFX_SCALE block
        // of real pixels. Overwrites part of the placeholder fill above.
        for gy in 0..GFX_HEIGHT {
            for gx in 0..GFX_WIDTH {
                let on = cpu.gfx[gy * GFX_WIDTH + gx] != 0;
                let color = if on { 0xFFFFFF } else { 0x000000 };

                let x_start = gx * GFX_SCALE;
                let y_start = gy * GFX_SCALE;

                for y in y_start..y_start + GFX_SCALE {
                    for x in x_start..x_start + GFX_SCALE {
                        buffer[y * WIDTH + x] = color;
                    }
                }
            }
        }

        // Clear all keys before checking this frame's mouse state, so a key
        // only stays "pressed" while the mouse is actively held over it.
        for key in cpu.keypad.iter_mut() {
            *key = false;
        }

        // Hit-test the mouse against the keypad grid and mark the key under
        // the cursor as pressed, if the left button is held.
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

        // Draw all 16 keypad buttons: green while held, dark gray otherwise.
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
                // index y * WIDTH + x.
                for y in y_start..y_end {
                    for x in x_start..x_end {
                        buffer[y * WIDTH + x] = color;
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
                                    buffer[y * WIDTH + x] = label_color;
                                }
                            }
                        }
                    }
                }
            }
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}