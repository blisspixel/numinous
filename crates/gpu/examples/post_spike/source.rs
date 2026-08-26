pub(crate) fn source_frame(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = vec![0; width as usize * height as usize * 4];
    let width_scale = 1.0 / width.max(1) as f32;
    let height_scale = 1.0 / height.max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let normalized_x = x as f32 * width_scale;
            let normalized_y = y as f32 * height_scale;
            let dx = normalized_x - 0.52;
            let dy = normalized_y - 0.46;
            let glow = (-42.0 * (dx * dx + dy * dy)).exp();
            let index = (y as usize * width as usize + x as usize) * 4;
            rgba[index] = ((0.05 + normalized_x * 0.35 + glow * 0.85).min(1.0) * 255.0) as u8;
            rgba[index + 1] = ((0.03 + normalized_y * 0.22 + glow * 0.70).min(1.0) * 255.0) as u8;
            rgba[index + 2] = ((0.12 + (1.0 - normalized_x) * 0.26 + glow).min(1.0) * 255.0) as u8;
            rgba[index + 3] = 255;
            let vertical_beam =
                x.abs_diff(width * 3 / 8) <= 1 && (height / 5..height * 4 / 5).contains(&y);
            let horizontal_beam =
                y.abs_diff(height * 2 / 3) <= 1 && (width / 7..width * 5 / 7).contains(&x);
            if vertical_beam || horizontal_beam {
                rgba[index] = 255;
                rgba[index + 1] = 232;
                rgba[index + 2] = 176;
            }
        }
    }
    rgba
}
