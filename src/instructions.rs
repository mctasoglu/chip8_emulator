use crate::cpu::Chip8Cpu;

// Decode one opcode into its fields, then dispatch to the matching
// instruction method. Unrecognized/unimplemented opcodes just get logged.
pub fn handle_instruction(cpu: &mut Chip8Cpu, opcode: u16) {
    let x = ((opcode & 0x0F00) >> 8) as usize;
    let y = ((opcode & 0x00F0) >> 4) as usize;
    let n = (opcode & 0x000F) as u8;
    let nn = (opcode & 0x00FF) as u8;
    let nnn = opcode & 0x0FFF;

    match (opcode & 0xF000) >> 12 {
        0x0 => match opcode {
            0x00E0 => cpu.cls(),
            0x00EE => cpu.ret(),
            _ => println!("Unimplemented opcode: {:04X}", opcode),
        },
        0x1 => cpu.jp(nnn),
        0x2 => cpu.call(nnn),
        0x3 => cpu.se_vx_byte(x, nn),
        0x4 => cpu.sne_vx_byte(x, nn),
        0x5 => match n {
            0x0 => cpu.se_vx_vy(x, y),
            _ => println!("Unimplemented opcode: {:04X}", opcode),
        },
        0x6 => cpu.ld_vx_byte(x, nn),
        0x7 => cpu.add_vx_byte(x, nn),
        0x8 => match n {
            0x0 => cpu.ld_vx_vy(x, y),
            0x1 => cpu.or_vx_vy(x, y),
            0x2 => cpu.and_vx_vy(x, y),
            0x3 => cpu.xor_vx_vy(x, y),
            0x4 => cpu.add_vx_vy(x, y),
            0x5 => cpu.sub_vx_vy(x, y),
            0x6 => cpu.shr_vx(x),
            0x7 => cpu.subn_vx_vy(x, y),
            0xE => cpu.shl_vx(x),
            _ => println!("Unimplemented opcode: {:04X}", opcode),
        },
        0x9 => match n {
            0x0 => cpu.sne_vx_vy(x, y),
            _ => println!("Unimplemented opcode: {:04X}", opcode),
        },
        0xA => cpu.ld_i(nnn),
        0xC => cpu.rnd(x, nn),
        0xD => cpu.drw(x, y, n),
        0xE => match nn {
            0x9E => cpu.skp(x),
            0xA1 => cpu.sknp(x),
            _ => println!("Unimplemented opcode: {:04X}", opcode),
        },
        0xF => match nn {
            0x07 => cpu.ld_vx_dt(x),
            0x0A => cpu.ld_vx_k(x),
            0x15 => cpu.ld_dt_vx(x),
            0x18 => cpu.ld_st_vx(x),
            0x29 => cpu.ld_f_vx(x),
            0x33 => cpu.ld_b_vx(x),
            _ => println!("Unimplemented opcode: {:04X}", opcode),
        },
        _ => println!("Unimplemented opcode: {:04X}", opcode),
    }
}
