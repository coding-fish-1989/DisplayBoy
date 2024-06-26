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

use crate::shader_support;

use image::{GenericImage, GenericImageView, Luma, Rgb, Rgba, RgbaImage};
use shader_support::*;

#[inline(always)]
fn load_alpha_checked(buff: &AlphaImage, x: i32, y: i32, width: u32, height: u32) -> f32 {
    if x >= width as i32 || y >= height as i32 || x < 0 || y < 0 {
        return 0.0;
    }
    unsafe { buff.unsafe_get_pixel(x as u32, y as u32)[0] }
}

#[inline(always)]
fn apply_threshold_kernel(
    l: f32,
    x: u32,
    y: u32,
    kernel: &[[[f32; 3]; 4]; 4],
    adjustment: &GbColorAdjustment,
) -> f32 {
    let x = x % 4;
    let y = y % 4;
    let thresholds = kernel[x as usize][y as usize];
    if l <= thresholds[0] {
        if adjustment.invert {
            0.07
        } else {
            3.0 / 3.0
        }
    } else if l <= thresholds[1] {
        if adjustment.invert {
            1.0 / 3.0
        } else {
            2.0 / 3.0
        }
    } else if l <= thresholds[2] {
        if adjustment.invert {
            2.0 / 3.0
        } else {
            1.0 / 3.0
        }
    } else {
        if adjustment.invert {
            3.0 / 3.0
        } else {
            0.07
        }
    }
}

#[inline(always)]
fn apply_color_adjustments_threshold(
    mid_threshold: f32,
    adjustment: &GbColorAdjustment,
) -> [f32; 3] {
    // Input threshold is between 0 and 1, and this will adjust it to the range of 0.25 and 0.75.
    // This is because this is trying to quantize color to 4 colors, and the lower quantization level should be 0.25 with contrast of 1 and brightness of 1.
    let threshold = (mid_threshold - 0.5) * 0.25 + 0.5;
    let range = 0.25 / adjustment.contrast.max(0.01);
    let border_error = 0.03;
    [
        ((threshold - range + border_error) / adjustment.brightness).clamp(0.0, 1.0),
        ((threshold + border_error) / adjustment.brightness).clamp(0.0, 1.0),
        ((threshold + range + border_error) / adjustment.brightness).clamp(0.0, 1.0),
    ]
}

#[inline(always)]
fn threshold_bayer_dither(x: u32, y: u32, adjustment: &GbColorAdjustment) -> [f32; 3] {
    // 4x4 Bayer
    // https://en.wikipedia.org/wiki/Ordered_dithering
    // Not exactly sure if this is what GB Camera uses, but perceptually it looks similar.
    let dither = [
        [0.0, 0.5, 0.125, 0.625],
        [0.75, 0.25, 0.875, 0.375],
        [0.1875, 0.6875, 0.0625, 0.5625],
        [0.9375, 0.4375, 0.8125, 0.3125],
    ];
    let threshold = dither[x as usize % 4][y as usize % 4];
    apply_color_adjustments_threshold(threshold, adjustment)
}

fn build_threshold_bayer_dither_kernel(adjustment: &GbColorAdjustment) -> [[[f32; 3]; 4]; 4] {
    let mut kernel = [[[0.0; 3]; 4]; 4];
    for y in 0..4 {
        for x in 0..4 {
            kernel[x][y] = threshold_bayer_dither(x as u32, y as u32, adjustment);
        }
    }
    kernel
}

fn build_threshold_default_kernel(adjustment: &GbColorAdjustment) -> [[[f32; 3]; 4]; 4] {
    let mut kernel = [[[0.0; 3]; 4]; 4];
    for y in 0..4 {
        for x in 0..4 {
            kernel[x][y] = apply_color_adjustments_threshold(0.5, adjustment)
        }
    }
    kernel
}

fn build_threshold_kernel(adjustment: &GbColorAdjustment) -> [[[f32; 3]; 4]; 4] {
    if adjustment.dither {
        build_threshold_bayer_dither_kernel(adjustment)
    } else {
        build_threshold_default_kernel(adjustment)
    }
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
            unsafe {
                to_buff.unsafe_put_pixel(
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

pub struct GbColorAdjustment {
    pub dither: bool,
    pub brightness: f32,
    pub contrast: f32,
    pub invert: bool,
    pub edge_enhancement_level: f32,
}

pub fn gb_mono(
    img: &FloatImage,
    profile: &GbDisplayProfile,
    adjustment: &GbColorAdjustment,
) -> RgbaImage {
    let (src_width, src_height) = (img.width(), img.height());

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

    // Quantize to alpha
    let quantized_img = AlphaImage::from_fn(src_width, src_height, |x, y| unsafe {
        let c = img.unsafe_get_pixel(x, y);
        let l = c.luminance();
        let l = if l <= (216.0 / 24389.0) {
            l * (24389.0 / 27.0)
        } else {
            l.powf(1.0 / 3.0) * 116.0 - 16.0
        };
        let l = (l / 100.0).clamp(0.0, 1.0);
        Luma([l])
    });

    // Apply adjustments
    let mut adjusted_img = AlphaImage::new(src_width, src_height);
    {
        let threshold_kernel = build_threshold_kernel(adjustment);
        let edge_enhancement_level = adjustment.edge_enhancement_level;
        let mut y_up = 0;
        for y in 0..src_height {
            let y_down = (y + 1).min(src_height - 1);

            let mut x_left = 0;
            for x in 0..src_width {
                unsafe {
                    let mut l = quantized_img.unsafe_get_pixel(x, y)[0];

                    // Apply edge enhancement
                    // https://github.com/LIJI32/SameBoy/blob/master/Core/camera.c
                    if edge_enhancement_level > 0.0 {
                        let x_right = (x + 1).min(src_width - 1);
                        l += (l * 4.0) * edge_enhancement_level;
                        l -= quantized_img.unsafe_get_pixel(x_left, y)[0] * edge_enhancement_level;
                        l -= quantized_img.unsafe_get_pixel(x_right, y)[0] * edge_enhancement_level;
                        l -= quantized_img.unsafe_get_pixel(x, y_up)[0] * edge_enhancement_level;
                        l -= quantized_img.unsafe_get_pixel(x, y_down)[0] * edge_enhancement_level;
                    }

                    let alpha = apply_threshold_kernel(l, x, y, &threshold_kernel, &adjustment)
                        * fg_opacity;
                    adjusted_img.unsafe_put_pixel(x, y, Luma([alpha]));
                }

                x_left = x;
            }

            y_up = y;
        }
    }

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

    // Upscale with grid line
    let mut bg_ping_buff = AlphaImage::from_fn(out_width, out_height, |x, y| {
        let x = x as i32 - margin as i32;
        let y = y as i32 - margin as i32;

        // Margin
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return Luma([0.0]);
        }

        // Gap between pixels. This code only works for resolution scale of 5.
        if (x % 5) >= 4 || (y % 5) >= 4 {
            return Luma([0.0]);
        }

        let x = x / scale as i32;
        let y = y / scale as i32;
        let alpha = unsafe { adjusted_img.unsafe_get_pixel(x as u32, y as u32)[0] };
        Luma([alpha])
    });

    let mut bg_pong_buff = AlphaImage::new(out_width, out_height);

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
            let color = bg.mult_f(1.0 - shadow);
            // Alpha blend foreground
            let opacity = load_alpha_checked(&fg_buff, x as i32, y as i32, out_width, out_height);
            let color = fg.mult_f(opacity).add(color.mult_f(1.0 - opacity));
            // Gamma compression
            let color = color.to_gamma();
            unsafe {
                out.unsafe_put_pixel(
                    x as u32,
                    y as u32,
                    Rgba([
                        float_to_byte(color[0]),
                        float_to_byte(color[1]),
                        float_to_byte(color[2]),
                        255,
                    ]),
                );
            }
        }
    }
    out
}
