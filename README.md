# CHIP-8 Emulator (Rust)

A CHIP-8 interpreter written in Rust, built from scratch as a project to learn Rust. See `AGENTS.md` for how this project is being developed (pair-programming style, with an AI coding assistant).

## Status: work in progress

Currently implemented:
- CPU state: 4KB memory, 16 general-purpose registers (`V0`-`VF`), index register `I`, program counter `PC`, a 64x32 display buffer, a keypad state array. (Stack and delay/sound timers exist as fields but aren't wired up to any instructions yet.)
- A real `PC`-driven fetch-execute loop, running a fixed number of cycles per rendered frame (see `CYCLES_PER_FRAME` in `src/main.rs`) - so jumps and loops in a ROM behave correctly and continuously, not just a one-shot pass through the file.
- 6 of the ~34 CHIP-8 instructions:
  - `00E0` - clear the display
  - `1NNN` - jump
  - `6XNN` - set register `VX`
  - `7XNN` - add to register `VX`
  - `ANNN` - set index register `I`
  - `DXYN` - draw a sprite
- An interactive, clickable on-screen keypad (mouse-driven, labeled with the standard COSMAC VIP hex layout), rendered in the same window alongside the game display.
- Unrecognized/unimplemented opcodes are logged to the console instead of crashing.

## Not yet implemented

- Most of the instruction set: arithmetic (`8XY_`), skip/compare (`3XNN`/`4XNN`/`5XY0`/`9XY0`), subroutine calls (`CALL`/`RET` and the call stack), timers, random numbers, keyboard-input instructions, BCD conversion, and bulk register/memory load-store.
- Wiring the on-screen keypad's state into the CPU - clicking keys doesn't affect emulation yet, since the input opcodes (`EX9E`, `EXA1`, `FX0A`) aren't built.
- A `Chip8Cpu::new()` constructor - CPU state is currently built with a struct literal directly in `main`.

## Running

```sh
cargo run -- path/to/rom.ch8
```

(The `--` tells `cargo` that what follows is an argument to the program, not to `cargo` itself.) Opens a native window via `minifb`, so this needs to run somewhere with a display attached.

Example, using one of the included test ROMs:

```sh
cargo run -- test_rom.ch8
```

## Project layout

- `src/main.rs` - everything: the `Chip8Cpu` struct, instruction implementations, the fetch-execute loop, window setup, display rendering, and the keypad UI.
- `*.ch8` - test ROMs:
  - `maze_rom.ch8`
  - `test_rom.ch8` - a small hand-built ROM exercising exactly the 6 implemented opcodes.
  - `ibm_logo.ch8` - the classic CHIP-8 "IBM logo" test ROM (uses the same 6 opcodes).

## Dependencies

- [`minifb`](https://crates.io/crates/minifb) - minimal cross-platform window + raw pixel framebuffer, used for both the CHIP-8 display and the keypad UI.
- `macroquad` is listed in `Cargo.toml` but not currently used.
