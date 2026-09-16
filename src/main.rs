mod cpu;
mod display;
mod instructions;
mod keypad;

use cpu::Chip8Cpu;
use instructions::handle_instruction;
use minifb::{Key, Window, WindowOptions};
use std::env;
use std::fs;

// Total window size: left side is the game display, right side is the keypad.
pub const WIDTH: usize = 1000;
pub const HEIGHT: usize = 500;

// How many fetch-execute cycles to run per rendered frame. At 60fps this is
// roughly a 600Hz CPU clock - runs the program continuously while still
// refreshing the display every frame, rather than all at once up front.
const CYCLES_PER_FRAME: usize = 10;

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

    let mut cpu = Chip8Cpu::new();

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

        // Count both timers down once per frame (~60Hz, matching the
        // window's target frame rate) - independent of how many
        // instructions just ran above.
        cpu.tick_timers();

        // Placeholder: fill the whole buffer with one flat color (game display
        // isn't wired up to `cpu.gfx` yet). Gets partially overwritten below
        // by the keypad drawing.
        for i in buffer.iter_mut() {
            *i = 277; // write something more funny here!
        }

        display::render(&cpu, &mut buffer, WIDTH);

        keypad::update(&mut cpu, &window);
        keypad::draw(&cpu, &mut buffer, WIDTH);

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}
