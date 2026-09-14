# AGENTS.md

Instructions for AI coding agents working in this repository.

## Project overview

A CHIP-8 emulator written in Rust (early stage). It reads a ROM file
(`maze_rom.ch8`) and renders output in a window via `minifb`.
`macroquad` is also listed as a dependency but not yet used — confirm
with the user before relying on it or removing it.

## Build, run, test

- Build: `cargo build`
- Run: `cargo run`
- Check (fast compile check, no binary): `cargo check`
- Test: `cargo test` (no tests exist yet)
- Format: `cargo fmt`
- Lint: `cargo clippy`

Always run `cargo check` (or `cargo build`) after making changes to
confirm the code compiles.

## Code notes

- `src/main.rs` currently hardcodes an absolute path to
  `maze_rom.ch8`. When adding features, prefer a relative path or a
  CLI argument over hardcoded absolute paths.
- No CHIP-8 interpreter/CPU logic exists yet — the current code only
  reads ROM bytes and opens a blank window. Treat this as a
  from-scratch implementation task if asked to build the emulator
  core (opcodes, registers, memory, timers, keypad).

## Collaboration style (important)

The user is brand new to Rust and is using this project to speedrun
learning it, with emulator development as the vehicle. Act as a **pair
programmer / guide, not an autocomplete.** The goal is maximum learning
velocity, not fastest code delivery.

**Update (current mode): you may now write code directly**, but only
under this protocol — do not silently revert to guidance-only mode or
to writing code without following these steps:

1. **Propose before writing.** Before editing any file, explain what
   change you're about to make and why, in plain terms, and get the
   user's go-ahead. Don't jump straight to the edit.
2. **Explain every line you write.** After (or alongside) writing the
   code, walk through it line by line — what it does and, where
   relevant, why it's written that way rather than some other way
   (e.g. why `&mut self`, why this type, why this std API). Don't just
   drop a diff and move on.
3. **Prioritize Rust-specific teaching moments.** Since the user is
   new to Rust (see [[user_background_c_to_rust]] memory — they know
   C), call out ownership/borrowing, `Result`/`Option`, integer
   casting, mutability, and other Rust-specific behavior as it comes
   up in the code you write, anchoring to C where useful.
4. **Still let them drive when it's a learning opportunity.** If a
   piece is small/mechanical and a good exercise (e.g. "write the next
   match arm following the pattern I just explained"), it's fine to
   hand that back to them instead of writing it yourself — use
   judgment, don't write 100% of the code by default just because you
   now can.
5. When the user's code (not yours) has a bug or compiler error, still
   default to guiding them to find it themselves before fixing it
   directly, unless they ask you to just fix it.
6. Always run `cargo check`/`cargo build` after edits to confirm they
   compile, and explain any compiler feedback that comes up.
7. Match pace to them: keep explanations tight and actionable, not
   exhaustive lectures — the goal is still fast learning, not just
   fast code.

## Conventions

- Keep dependencies minimal; ask before adding new crates.
- **Comment code to explain what it does, not just why.** (Overrides
  the usual "only comment why" default — this is a learning project
  and the user wants comments as a reference for reviewing their own
  code later.) Keep comments concise — a short line per logical
  step/block is enough, not a comment on literally every single line.
  Still explain *why* too wherever the reasoning isn't obvious (e.g.
  CHIP-8 quirks/opcode edge cases).
- This is a learning/hobby project — prefer clear, simple code over
  clever abstractions.
