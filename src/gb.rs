/*
    DisplayBoy

    Copyright (C) 2024 coding-fish-1989

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::{scaling, shader_support};

use image::{Luma, Rgb, Rgba, RgbaImage};
use shader_support::*;
use scaling::*;

#[inline(always)]
fn load_alpha_checked(buff: &AlphaImage, x: i32, y: i32, width: u32, height: u32) -> f32 {
    if x >= width as i32 || y >= height as i32 || x < 0 || y < 0 {
        return 0.0;
    }
    buff.get_pixel(x as u32, y as u32)[0]
}

fn bilinear_scale_alpha(
    from_width: u32,
    from_height: u32,
    to_width: u32,
    to_height: u32,
    from_buff: &AlphaImage,
    to_buff: &mut AlphaImage,
) {
    let x_ratio = from_width as f32 / to_width as f32;
    let y_ratio = from_height as f32 / to_height as f32;
    for y in 0..to_height {
        for x in 0..to_width {
            let xf = x as f32 * x_ratio;
            let yf = y as f32 * y_ratio;
            let xi = xf as i32;
            let yi = yf as i32;
            let xf = xf - xi as f32;
            let yf = yf - yi as f32;
            let a = load_alpha_checked(from_buff, xi, yi, from_width, from_height);
            let b = load_alpha_checked(from_buff, xi + 1, yi, from_width, from_height);
            let c = load_alpha_checked(from_buff, xi, yi + 1, from_width, from_height);
            let d = load_alpha_checked(from_buff, xi + 1, yi + 1, from_width, from_height);
            to_buff.put_pixel(
                x as u32,
                y as u32,
                Luma([((1.0 - xf) * (1.0 - yf) * a
                    + xf * (1.0 - yf) * b
                    + (1.0 - xf) * yf * c
                    + xf * yf * d)]),
            );
        }
    }
}

#[inline(always)]
fn quantize_gb(l: f32) -> f32 {
    let border_error = 0.03;
    if l <= 0.25 + border_error {
        return 1.0;
    } else if l <= 0.5 + border_error {
        return 0.66666667;
    } else if l <= 0.75 + border_error {
        return 0.33333333;
    }
    0.07
}

#[inline(always)]
fn apply_kernel_3(values: &[f32; 3], kernel: [f32; 2]) -> f32 {
    let mut g = 0.0;
    g += values[0] * kernel[0];
    g += values[1] * kernel[1];
    g += values[2] * kernel[0];
    g
}

#[inline(always)]
fn apply_kernel_7(values: &[f32; 7], kernel: [f32; 4]) -> f32 {
    let mut g = 0.0;
    g += values[0] * kernel[0];
    g += values[1] * kernel[1];
    g += values[2] * kernel[2];
    g += values[3] * kernel[3];
    g += values[4] * kernel[2];
    g += values[5] * kernel[1];
    g += values[6] * kernel[0];
    g
}

pub struct GbDisplayProfile {
    pub foreground_r: f32,
    pub foreground_g: f32,
    pub foreground_b: f32,
    pub foreground_a: f32,
    pub background_r: f32,
    pub background_g: f32,
    pub background_b: f32,
}

pub fn gb_mono(img: RgbaImage, src_scale: ScaleInfo, profile: GbDisplayProfile) -> RgbaImage {
    let src_width = img.width();
    let src_height = img.height();

    // Color configurations
    let fg = Rgb::<f32>([
        profile.foreground_r,
        profile.foreground_g,
        profile.foreground_b,
    ])
    .to_linear();
    let fg_opacity = profile.foreground_a;
    let bg = Rgb::<f32>([
        profile.background_r,
        profile.background_g,
        profile.background_b,
    ])
    .to_linear();

    // Need to accommodate for non-integer scaling
    let (target_width, target_height) =
        calculate_scaled_buffer_size(src_width, src_height, &src_scale);

    // Quantize to alpha, downscale to real device resolution, and store
    let mut buff = AlphaImage::new(target_width, target_height);
    let x_target_half_texel = 1.0 / (target_width as f32 * 2.0);
    let y_target_half_texel = 1.0 / (target_height as f32 * 2.0);
    for y in 0..target_height {
        for x in 0..target_width {
            // Nearest neighbor downscale
            let x_coord = x as f32 / target_width as f32 + x_target_half_texel;
            let y_coord = y as f32 / target_height as f32 + y_target_half_texel;
            let x_src = (x_coord * src_width as f32).floor() as u32;
            let y_src = (y_coord * src_height as f32).floor() as u32;
            let c = img.get_pixel(x_src, y_src);
            let c = rgba_u8_to_rgb_f32(*c).to_linear();
            let l = c.luminance();
            let l = if l <= (216.0 / 24389.0) {
                l * (24389.0 / 27.0)
            } else {
                l.powf(1.0 / 3.0) * 116.0 - 16.0
            };
            let l = l / 100.0;
            let alpha = quantize_gb(l) * fg_opacity;
            buff.put_pixel(x, y, Luma([alpha]));
        }
    }

    let (src_width, src_height) = (target_width, target_height);

    // Don't change this without revising pretty much everything after this
    let scale = 5;

    let width = src_width * scale;
    let height = src_height * scale;

    // Final output should have GB LCD margin
    // This margin also accounts for the edge smear and shadow blur expansion
    let native_margin = 5;
    let margin = native_margin * scale;
    let out_width = width + margin * 2;
    let out_height = height + margin * 2;

    // Prepare buffers for separable gaussian blur
    // Should be about 2.46 MB on memory per buffer for full GB resolution with margin above
    let mut bg_ping_buff = AlphaImage::new(out_width, out_height);
    let mut bg_pong_buff = AlphaImage::new(out_width, out_height);

    // Upscale with grid line
    for y in 0..height {
        for x in 0..width {
            // Gap between pixels. This code only works for resolution scale of 5.
            if (x % 5) >= 4 || (y % 5) >= 4 {
                continue;
            }

            bg_ping_buff.put_pixel(
                x + margin as u32,
                y + margin as u32,
                *buff.get_pixel(x / scale as u32, y / scale as u32),
            );
        }
    }

    // Apply small blur to smear the pixel edges
    {
        // Subjective smear kernel
        let kernel = [0.1, 0.8];

        // Horizontal gaussian blur pass
        for y in 0..out_height {
            for x in 0..out_width {
                let g = apply_kernel_3(
                    &[
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 - 1,
                            y as i32,
                            out_width,
                            out_height,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 + 0,
                            y as i32,
                            out_width,
                            out_height,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 + 1,
                            y as i32,
                            out_width,
                            out_height,
                        ),
                    ],
                    kernel,
                );
                bg_pong_buff.put_pixel(x, y, Luma([g]));
            }
        }

        std::mem::swap(&mut bg_ping_buff, &mut bg_pong_buff);

        // Vertical gaussian blur pass
        for y in 0..out_height {
            for x in 0..out_width {
                let g = apply_kernel_3(
                    &[
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 - 1,
                            out_width,
                            out_height,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 + 0,
                            out_width,
                            out_height,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 + 1,
                            out_width,
                            out_height,
                        ),
                    ],
                    kernel,
                );
                bg_pong_buff.put_pixel(x, y, Luma([g]));
            }
        }

        std::mem::swap(&mut bg_ping_buff, &mut bg_pong_buff);
    }

    // Gaussian blur result from above is now kept as a foreground buffer
    let fg_buff = bg_ping_buff;

    // Allocate a new buffer for ping buffer, since it's been used for foreground buffer
    bg_ping_buff = AlphaImage::new(out_width, out_height);

    // Apply larger blur for shadowing:
    // - Downsamples the foreground buffer to approximately half the size
    // - Applies separable gaussian blur
    // - Upsamples the buffer back to the original size
    // This is done to avoid using a massive gaussian blur kernel.

    // Half the size, but round up. This is to avoid losing pixel data.
    let out_width_small = (out_width + 1) / 2;
    let out_height_small = (out_height + 1) / 2;

    // Downsample
    bilinear_scale_alpha(
        out_width,
        out_height,
        out_width_small,
        out_height_small,
        &fg_buff,
        &mut bg_ping_buff,
    );

    {
        // Gaussian kernel
        let kernel = [0.006, 0.061, 0.241, 0.383];

        // Horizontal gaussian blur pass
        // It takes in blurred foreground buffer as an input to slightly increase the blur radius
        for y in 0..out_height_small {
            for x in 0..out_width_small {
                let g = apply_kernel_7(
                    &[
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 - 3,
                            y as i32,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 - 2,
                            y as i32,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 - 1,
                            y as i32,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 + 0,
                            y as i32,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 + 1,
                            y as i32,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 + 2,
                            y as i32,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32 + 3,
                            y as i32,
                            out_width_small,
                            out_height_small,
                        ),
                    ],
                    kernel,
                );
                bg_pong_buff.put_pixel(x as u32, y as u32, Luma([g]));
            }
        }

        std::mem::swap(&mut bg_ping_buff, &mut bg_pong_buff);

        // Vertical gaussian blur pass
        for y in 0..out_height_small {
            for x in 0..out_width_small {
                let g = apply_kernel_7(
                    &[
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 - 3,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 - 2,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 - 1,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 + 0,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 + 1,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 + 2,
                            out_width_small,
                            out_height_small,
                        ),
                        load_alpha_checked(
                            &bg_ping_buff,
                            x as i32,
                            y as i32 + 3,
                            out_width_small,
                            out_height_small,
                        ),
                    ],
                    kernel,
                );
                bg_pong_buff.put_pixel(x as u32, y as u32, Luma([g]));
            }
        }

        std::mem::swap(&mut bg_ping_buff, &mut bg_pong_buff);
    }

    // Upsample
    bilinear_scale_alpha(
        out_width_small,
        out_height_small,
        out_width,
        out_height,
        &bg_ping_buff,
        &mut bg_pong_buff,
    );

    let bg_shadow_buff = bg_pong_buff;

    let shadow_opacity = 0.5;
    let shadow_offset = 1;

    let mut out = RgbaImage::new(out_width as u32, out_height as u32);
    for y in 0..out_height {
        for x in 0..out_width {
            // Background shadowing
            let shadow = load_alpha_checked(
                &bg_shadow_buff,
                x as i32 - shadow_offset,
                y as i32 - shadow_offset,
                out_width,
                out_height,
            );
            let shadow = shadow * shadow_opacity;
            let c = bg.mult_f(1.0 - shadow);
            // Alpha blend foreground
            let opacity = load_alpha_checked(&fg_buff, x as i32, y as i32, out_width, out_height);
            let c = fg.mult_f(opacity).add(c.mult_f(1.0 - opacity));
            // Gamma compression
            let c = c.to_gamma();
            out.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    float_to_byte(c[0]),
                    float_to_byte(c[1]),
                    float_to_byte(c[2]),
                    255,
                ]),
            );
        }
    }
    out
}
