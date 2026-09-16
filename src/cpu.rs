// The standard CHIP-8 built-in font set: one 4x5-pixel sprite per hex digit
// (0-F), same format DXYN draws (each byte is one row, top 4 bits are the
// pixels, read MSB-first). This is the same data a real interpreter loads
// into memory for the FX29 instruction later - the keypad module also
// reuses it to draw readable labels on the keypad buttons.
pub const FONT_SET: [[u8; 5]; 16] = [
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

// Where the font set gets loaded into memory. Low memory (below 0x200,
// where ROMs are loaded) is conventionally used for interpreter-owned data
// like this on real CHIP-8 hardware.
pub const FONT_START: u16 = 0x50;

// The CHIP-8 virtual machine's state: everything an interpreter needs to
// fetch, decode, and execute instructions.
pub struct Chip8Cpu {
    pub memory: [u8; 4096],       // 4KB of addressable memory (program + data).
    pub registers: [u8; 16],      // General-purpose registers V0-VF.
    pub i: u16,                   // Index register: holds an address (e.g. for sprite data).
    pub pc: u16,                  // Program counter: address of the next instruction to fetch.
    pub gfx: [u8; 64 * 32],       // Display buffer, one byte per pixel (64x32 monochrome).
    pub delay_timer: u8,          // Counts down at 60Hz; used for game timing.
    pub sound_timer: u8,          // Counts down at 60Hz; beeps while nonzero.
    pub stack: [u16; 16],         // Return addresses for CALL/RET (16 levels deep).
    pub stack_pointer: u8,        // Index of the next free slot in `stack`.
    pub keypad: [bool; 16],       // Which of the 16 hex keys (0x0-0xF) are currently held.
}

impl Chip8Cpu {
    //Constructor
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: [0; 4096],
            registers: [0; 16],
            i: 0,
            pc: 0x200, // Start at 0x200, where programs are loaded.
            gfx: [0; 64 * 32],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            stack_pointer: 0,
            keypad: [false; 16],
        };

        // Load the built-in font set into memory starting at FONT_START, so
        // FX29 can point I at a specific digit's glyph later.
        for (digit, glyph) in FONT_SET.iter().enumerate() {
            let addr = FONT_START as usize + digit * 5;
            cpu.memory[addr..addr + 5].copy_from_slice(glyph);
        }

        cpu
    }

    // 00E0: clear the display.
    pub fn cls(&mut self) {
        self.gfx = [0; 64 * 32];
    }

    // 1NNN: jump to address NNN.
    pub fn jp(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    // 2NNN: call the subroutine at NNN. Push the current `pc` (the address
    // fetch() already advanced to, i.e. "come back here after RET") onto the
    // stack, then jump.
    pub fn call(&mut self, nnn: u16) {
        self.stack[self.stack_pointer as usize] = self.pc;
        self.stack_pointer += 1;
        self.pc = nnn;
    }

    // 00EE: return from the current subroutine. Pop the return address the
    // matching CALL pushed, and jump back to it.
    pub fn ret(&mut self) {
        self.stack_pointer -= 1;
        self.pc = self.stack[self.stack_pointer as usize];
    }

    // 3XNN: skip the next instruction if VX == NN.
    pub fn se_vx_byte(&mut self, x: usize, nn: u8) {
        if self.registers[x] == nn {
            self.pc += 2;
        }
    }

    // 4XNN: skip the next instruction if VX != NN.
    pub fn sne_vx_byte(&mut self, x: usize, nn: u8) {
        if self.registers[x] != nn {
            self.pc += 2;
        }
    }

    // 5XY0: skip the next instruction if VX == VY.
    pub fn se_vx_vy(&mut self, x: usize, y: usize) {
        if self.registers[x] == self.registers[y] {
            self.pc += 2;
        }
    }

    // 9XY0: skip the next instruction if VX != VY.
    pub fn sne_vx_vy(&mut self, x: usize, y: usize) {
        if self.registers[x] != self.registers[y] {
            self.pc += 2;
        }
    }

    // 6XNN: set VX = NN.
    pub fn ld_vx_byte(&mut self, x: usize, nn: u8) {
        self.registers[x] = nn;
    }

    // 7XNN: VX += NN. CHIP-8 defines this as wrapping on overflow (no carry
    // flag set), unlike 8XY4. Plain `+` would panic on overflow in debug
    // builds, so we need `wrapping_add` to get the "just wrap around" behavior.
    pub fn add_vx_byte(&mut self, x: usize, nn: u8) {
        self.registers[x] = self.registers[x].wrapping_add(nn);
    }

    // 8XY0: VX = VY.
    pub fn ld_vx_vy(&mut self, x: usize, y: usize) {
        self.registers[x] = self.registers[y];
    }

    // 8XY1: VX |= VY. (Note: some real CHIP-8 hardware also reset VF to 0 on
    // this instruction - a documented compatibility "quirk." We don't do
    // that here, matching the more common modern reference behavior.)
    pub fn or_vx_vy(&mut self, x: usize, y: usize) {
        self.registers[x] |= self.registers[y];
    }

    // 8XY2: VX &= VY.
    pub fn and_vx_vy(&mut self, x: usize, y: usize) {
        self.registers[x] &= self.registers[y];
    }

    // 8XY3: VX ^= VY.
    pub fn xor_vx_vy(&mut self, x: usize, y: usize) {
        self.registers[x] ^= self.registers[y];
    }

    // 8XY4: VX += VY, with carry. Unlike 7XNN, this DOES set VF: 1 if the
    // result overflowed 8 bits, else 0. `overflowing_add` (unlike
    // `wrapping_add`) hands back both the wrapped result AND whether it
    // overflowed, as a tuple - we need both pieces here.
    pub fn add_vx_vy(&mut self, x: usize, y: usize) {
        let (result, overflowed) = self.registers[x].overflowing_add(self.registers[y]);
        self.registers[x] = result;
        self.registers[0xF] = overflowed as u8;
    }

    // 8XY5: VX -= VY. VF is set to 1 if NO borrow occurred (VX >= VY before
    // the subtraction), 0 if there was a borrow (VX < VY) - the opposite
    // polarity from a carry flag, per the CHIP-8 spec.
    pub fn sub_vx_vy(&mut self, x: usize, y: usize) {
        let (result, borrowed) = self.registers[x].overflowing_sub(self.registers[y]);
        self.registers[x] = result;
        self.registers[0xF] = !borrowed as u8;
    }

    // 8XY7: VX = VY - VX (subtract in the other direction). Same VF
    // convention as 8XY5, just with the operands swapped.
    pub fn subn_vx_vy(&mut self, x: usize, y: usize) {
        let (result, borrowed) = self.registers[y].overflowing_sub(self.registers[x]);
        self.registers[x] = result;
        self.registers[0xF] = !borrowed as u8;
    }

    // 8XY6: shift VX right by 1. VF is set to the bit that got shifted out
    // (VX's original least-significant bit).
    pub fn shr_vx(&mut self, x: usize) {
        let dropped_bit = self.registers[x] & 0x1;
        self.registers[x] >>= 1;
        self.registers[0xF] = dropped_bit;
    }

    // 8XYE: shift VX left by 1. VF is set to the bit that got shifted out
    // (VX's original most-significant bit).
    pub fn shl_vx(&mut self, x: usize) {
        let dropped_bit = (self.registers[x] & 0x80) >> 7;
        self.registers[x] <<= 1;
        self.registers[0xF] = dropped_bit;
    }

    // CXNN: VX = (a random byte) & NN. ANDing with NN masks the random byte
    // down to whatever range the program wants (e.g. NN=0x01 for a random
    // bit, NN=0x0F for a random value 0-15).
    pub fn rnd(&mut self, x: usize, nn: u8) {
        let random_byte: u8 = rand::random();
        self.registers[x] = random_byte & nn;
    }

    // ANNN: set I = NNN.
    pub fn ld_i(&mut self, nnn: u16) {
        self.i = nnn;
    }

    // Read the 2-byte instruction at `pc`, advance `pc` past it, and return
    // the combined opcode. Advancing *before* returning (rather than after)
    // is what lets a jump instruction (which assigns directly to `self.pc`)
    // simply overwrite this advance instead of it being undone or reapplied.
    pub fn fetch(&mut self) -> u16 {
        let hi = self.memory[self.pc as usize] as u16;
        let lo = self.memory[self.pc as usize + 1] as u16; //This increments to get the following byte because we need to read 2 bytes for the opcode
        self.pc += 2;
        (hi << 8) | lo
    }

    // FX07: VX = delay_timer.
    pub fn ld_vx_dt(&mut self, x: usize) {
        self.registers[x] = self.delay_timer;
    }

    // FX15: delay_timer = VX.
    pub fn ld_dt_vx(&mut self, x: usize) {
        self.delay_timer = self.registers[x];
    }

    // FX18: sound_timer = VX.
    pub fn ld_st_vx(&mut self, x: usize) {
        self.sound_timer = self.registers[x];
    }

    // FX29: point I at the built-in font sprite for the hex digit in VX.
    // Each glyph is 5 bytes, stored back-to-back starting at FONT_START.
    pub fn ld_f_vx(&mut self, x: usize) {
        let digit = self.registers[x] as u16;
        self.i = FONT_START + digit * 5;
    }

    // FX33: store the BCD (binary-coded decimal) representation of VX at
    // memory[I], memory[I+1], memory[I+2] - hundreds, tens, ones digits.
    // E.g. VX=123 -> memory[I]=1, memory[I+1]=2, memory[I+2]=3. This is how
    // a game turns a raw byte (like a score) into separate decimal digits
    // it can then draw on screen, one FX29+DXYN pair per digit.
    pub fn ld_b_vx(&mut self, x: usize) {
        let value = self.registers[x];
        self.memory[self.i as usize] = value / 100;
        self.memory[self.i as usize + 1] = (value / 10) % 10;
        self.memory[self.i as usize + 2] = value % 10;
    }

    // Decrement both timers by 1, if nonzero. Meant to be called once per
    // rendered frame - the window targets 60fps, and both timers are
    // defined to count down at a fixed 60Hz, independent of how many
    // instructions actually execute.
    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    // DXYN: draw an N-row sprite (read from memory starting at I) at (VX, VY).
    // Each sprite row is a byte; each bit is one pixel, drawn via XOR.
    // VF is set to 1 if any pixel was flipped from on to off (a "collision").
    pub fn drw(&mut self, x: usize, y: usize, n: u8) {
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

    // EX9E: skip the next instruction if the key numbered VX is held.
    pub fn skp(&mut self, x: usize) {
        let key = self.registers[x] as usize;
        if self.keypad[key] {
            self.pc += 2;
        }
    }

    // EXA1: skip the next instruction if the key numbered VX is NOT held.
    pub fn sknp(&mut self, x: usize) {
        let key = self.registers[x] as usize;
        if !self.keypad[key] {
            self.pc += 2;
        }
    }

    // FX0A: block until some key is pressed, then store which one in VX.
    // If nothing is pressed this cycle, rewind `pc` by 2 so the same
    // instruction fetches and runs again next cycle - i.e. "wait here."
    pub fn ld_vx_k(&mut self, x: usize) {
        match self.keypad.iter().position(|&pressed| pressed) {
            Some(key) => self.registers[x] = key as u8,
            None => self.pc -= 2,
        }
    }
}
