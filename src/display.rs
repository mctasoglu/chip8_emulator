use crate::cpu::Chip8Cpu;

// CHIP-8's native display resolution, and how many real pixels each CHIP-8
// pixel gets scaled up to when drawn into the window's game area.
pub const GFX_WIDTH: usize = 64;
pub const GFX_HEIGHT: usize = 32;
pub const GFX_SCALE: usize = 9;

// Render cpu.gfx (the actual CHIP-8 display, 64x32) into the game area,
// scaling each CHIP-8 pixel up to a GFX_SCALE x GFX_SCALE block of real
// pixels, anchored at the top-left of `buffer`.
pub fn render(cpu: &Chip8Cpu, buffer: &mut [u32], width: usize) {
    for gy in 0..GFX_HEIGHT {
        for gx in 0..GFX_WIDTH {
            let on = cpu.gfx[gy * GFX_WIDTH + gx] != 0;
            let color = if on { 0xFFFFFF } else { 0x000000 };

            let x_start = gx * GFX_SCALE;
            let y_start = gy * GFX_SCALE;

            for y in y_start..y_start + GFX_SCALE {
                for x in x_start..x_start + GFX_SCALE {
                    buffer[y * width + x] = color;
                }
            }
        }
    }
}
