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

/*
   Based on CRT-interlaced (CRT Geom)

   Copyright (C) 2010-2012 cgwg, Themaister and DOLLS

   This program is free software; you can redistribute it and/or modify it
   under the terms of the GNU General Public License as published by the Free
   Software Foundation; either version 2 of the License, or (at your option)
   any later version.

   (cgwg gave their consent to have the original version of this shader
   distributed under the GPL in this message:

       http://board.byuu.org/viewtopic.php?p=26075#p26075

       "Feel free to distribute my shaders under the GPL. After all, the
       barrel distortion code was taken from the Curvature shader, which is
       under the GPL."
   )

   Changes from original:
   - Removes screen curvature
   - Changes dotmask to be source size dependent
   - Changes Lanczos to normal Lanczos2
   - And obviously written in Rust
*/

use crate::{scaling, shader_support};

use image::{Rgb, Rgba, RgbaImage};
use scaling::*;
use shader_support::*;

pub const CRT_MARGIN: u32 = 4;
pub const CRT_SCANLINE_WEIGHT: f32 = 0.3;
pub const CRT_LUM: f32 = 0.0;
pub const CRT_DOT_MASK: f32 = 0.05;
pub const CRT_GAMMA: f32 = 2.5;
pub const CRT_LANCZOS_SIZE: i32 = 2;

pub const CRT_PWR: f32 =
    1.0 / ((-0.7 * (1.0 - CRT_SCANLINE_WEIGHT) + 1.0) * (-0.5 * CRT_DOT_MASK + 1.0)) - 1.25;

#[inline(always)]
pub fn scanline_wid(color: Rgb<f32>) -> Rgb<f32> {
    let color2 = color.mult(color);
    let color4 = color2.mult(color2);
    color4.mult_f(2.0).add_f(2.0)
}

#[inline(always)]
pub fn scanline_weights(distance: f32, wid: Rgb<f32>) -> Rgb<f32> {
    wid.mult_f(0.5)
        .inv_sqrt()
        .mult_f(distance / CRT_SCANLINE_WEIGHT)
        .pow(wid)
        .mult_f(-1.0)
        .exp()
        .mult_f(CRT_LUM + 1.4)
        .div(wid.mult_f(0.2).add_f(0.6))
}

#[inline(always)]
pub fn crt_inv_gamma(col: Rgb<f32>) -> Rgb<f32> {
    let cir = col.sub_f(1.0);
    let cir = cir.mult(cir);
    lerp_color(
        col.sqrt(),
        (Rgb::<f32>([1.0, 1.0, 1.0]).sub(cir)).sqrt(),
        CRT_PWR,
    )
}

#[inline(always)]
fn apply_lut_rgb3d(col: Rgba<f32>, lut: &[Rgb<f32>; 32 * 32 * 32]) -> Rgb<f32> {
    let r = col[0];
    let g = col[1];
    let b = col[2];

    let x = (r * 31.0).floor() as usize;
    let y = (g * 31.0).floor() as usize;
    let z = (b * 31.0).floor() as usize;

    let dx = r * 31.0 - x as f32;
    let dy = g * 31.0 - y as f32;
    let dz = b * 31.0 - z as f32;

    let x_next = (x + 1).min(31);
    let y_next = (y + 1).min(31);
    let z_next = (z + 1).min(31);

    let c00 = lerp_color(
        lut[x + y * 32 + z * 32 * 32],
        lut[x_next + y * 32 + z * 32 * 32],
        dx,
    );
    let c01 = lerp_color(
        lut[x + y * 32 + z_next * 32 * 32],
        lut[x_next + y * 32 + z_next * 32 * 32],
        dx,
    );
    let c10 = lerp_color(
        lut[x + y_next * 32 + z * 32 * 32],
        lut[x_next + y_next * 32 + z * 32 * 32],
        dx,
    );
    let c11 = lerp_color(
        lut[x + y_next * 32 + z_next * 32 * 32],
        lut[x_next + y_next * 32 + z_next * 32 * 32],
        dx,
    );

    let c0 = lerp_color(c00, c10, dy);
    let c1 = lerp_color(c01, c11, dy);

    lerp_color(c0, c1, dz)
}

pub fn crt(
    img: RgbaImage,
    src_scale: ScaleInfo,
    scale: u32,
    explicit_aspect_ratio: bool,
    pixel_aspect_ratio: f32,
) -> RgbaImage {
    let lut_png = include_bytes!("crt_lut.png");
    let lut_img = image::load_from_memory(lut_png).unwrap().to_rgba8();
    let mut lut = [Rgb::<f32>([0.0, 0.0, 0.0]); 32 * 32 * 32];
    for z in 0..32 {
        for y in 0..32 {
            for x in 0..32 {
                let p = lut_img.get_pixel((x + z * 32) as u32, y as u32);
                lut[x + y * 32 + z * 32 * 32] = Rgb::<f32>([
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                ])
                .to_linear_from_gamma(CRT_GAMMA);
            }
        }
    }

    let load_buff = |x: i32, y: i32| -> Rgb<f32> {
        if y < 0 || y >= img.height() as i32 || x < 0 || x >= img.width() as i32 {
            return Rgb([0.0, 0.0, 0.0]);
        }
        let p = img.get_pixel(x as u32, y as u32);
        let p = Rgba::<f32>([
            p[0] as f32 / 255.0,
            p[1] as f32 / 255.0,
            p[2] as f32 / 255.0,
            1.0,
        ]);
        apply_lut_rgb3d(p, &lut)
    };

    let (src_width, src_height) = (img.width(), img.height());

    let (target_width, target_height) =
        calculate_scaled_buffer_size(src_width, src_height, &src_scale);

    // Automatic height padding for devices like SNES
    let vertical_padding = if target_height < 240 && target_height >= 224 {
        let total_pad = 240 - target_height;
        // Split padding evenly, with the remainder going to the bottom
        let pad_top = total_pad / 2;
        let pad_bottom = total_pad - pad_top;
        (pad_top, pad_bottom)
    } else {
        (0, 0) // No padding
    };

    let output_width_factor = if explicit_aspect_ratio {
        pixel_aspect_ratio
    } else if src_scale.respect_input_aspect_ratio {
        // The source image might be a different aspect ratio than the target
        // This can happen if the image was upscaled and then stretched to reflect the non square pixel.
        // The following code will respect the source image's aspect ratio and calculated the output width factor.
        let desired_aspect_ratio = src_width as f32 / src_height as f32;
        let source_aspect_ratio = target_width as f32 / target_height as f32;
        desired_aspect_ratio / source_aspect_ratio
    } else {
        1.0
    };

    let top_margin = CRT_MARGIN + vertical_padding.0;
    // The margin is defined to be in a unit after applying aspect ratio stretching
    let left_margin = (CRT_MARGIN as f32 / output_width_factor).ceil() as u32;

    // Create a source buffer with margins
    let (buff_width, buff_height) = (
        target_width + left_margin * 2,
        target_height + CRT_MARGIN * 2 + vertical_padding.0 + vertical_padding.1,
    );
    let mut buff = FloatImage::new(buff_width, buff_height);

    // Note that target_width and target_height are being used to scale here on purpose.
    // buff_width and buff_height are only the size of the output buffer, and does not affect the nearest neighbor scaling.
    let x_target_half_texel = 1.0 / (target_width as f32 * 2.0);
    let y_target_half_texel = 1.0 / (target_height as f32 * 2.0);
    for y in 0..buff_height {
        for x in 0..buff_width {
            // Nearest neighbor downscale
            let x_coord =
                (x as i32 - left_margin as i32) as f32 / target_width as f32 + x_target_half_texel;
            let y_coord =
                (y as i32 - top_margin as i32) as f32 / target_height as f32 + y_target_half_texel;
            let x_src = (x_coord * src_width as f32).floor() as i32;
            let y_src = (y_coord * src_height as f32).floor() as i32;
            buff.put_pixel(x, y, load_buff(x_src, y_src));
        }
    }

    let (src_width, src_height) = (buff_width, buff_height);
    let (width, height) = (
        (src_width as f32 * scale as f32 * output_width_factor).ceil() as u32,
        src_height * scale,
    );

    let src_width_f = src_width as f32;
    let src_height_f = src_height as f32;

    let load_buff = |x: i32, y: i32| -> Rgb<f32> {
        if y < 0 || y >= src_height as i32 || x < 0 || x >= src_width as i32 {
            return Rgb([0.0, 0.0, 0.0]);
        }
        *buff.get_pixel(x as u32, y as u32)
    };

    let out_texel_size_x = 1.0 / width as f32;
    let out_texel_size_y = 1.0 / height as f32;

    let src_texel_size_x = 1.0 / src_width as f32;

    // Precompute Lanczos weights
    let mut lanczos_weights =
        vec![[0.0; CRT_LANCZOS_SIZE as usize * 2usize + 1usize]; width as usize];
    for x in 0..width {
        let tex_coord_x = out_texel_size_x * (x as f32 + 0.5);
        let ratio_scale_x = tex_coord_x * src_width_f - 0.5;
        let xx = ratio_scale_x.floor();

        let mut sum = 0.0;
        let out_tex_coord_x = x as f32 * out_texel_size_x;
        let src_tex_coord_x = xx * src_texel_size_x;
        for lx in -CRT_LANCZOS_SIZE..=CRT_LANCZOS_SIZE {
            let src_sample_tex_coord_x = src_tex_coord_x + lx as f32 * src_texel_size_x;
            let d = clamp(
                (src_sample_tex_coord_x - out_tex_coord_x) * src_width_f,
                -CRT_LANCZOS_SIZE as f32,
                CRT_LANCZOS_SIZE as f32,
            );

            let mut w = 1.0;
            if d != 0.0 {
                let d = d * std::f32::consts::PI;
                w = (CRT_LANCZOS_SIZE as f32 * d.sin() * (d / (CRT_LANCZOS_SIZE as f32)).sin())
                    / (d * d);
            }

            lanczos_weights[x as usize][(lx + CRT_LANCZOS_SIZE) as usize] = w;
            sum += w;
        }

        for lx in -CRT_LANCZOS_SIZE..=CRT_LANCZOS_SIZE {
            lanczos_weights[x as usize][(lx + CRT_LANCZOS_SIZE) as usize] /= sum;
        }
    }

    let mut out = RgbaImage::new(width, height);

    for y in 0..height {
        let tex_coord_y = out_texel_size_y * (y as f32 + 0.5);

        let ratio_scale_y = tex_coord_y * src_height_f - 0.5;
        let yy = ratio_scale_y.floor();
        let uv_ratio_y = ratio_scale_y - yy;

        for x in 0..width {
            let tex_coord_x = out_texel_size_x * (x as f32 + 0.5);

            let ratio_scale_x = tex_coord_x * src_width_f - 0.5;
            let xx = ratio_scale_x.floor();
            let uv_ratio_x = ratio_scale_x - xx;

            let mut col = Rgb::<f32>([0.0, 0.0, 0.0]);
            let mut col2 = Rgb::<f32>([0.0, 0.0, 0.0]);

            // Horizontal only Lanczos using precomputed weights
            for lx in -CRT_LANCZOS_SIZE..=CRT_LANCZOS_SIZE {
                let w = lanczos_weights[x as usize][(lx + CRT_LANCZOS_SIZE) as usize];

                let val = load_buff(xx as i32 + lx, yy as i32);
                col = col.add(val.mult_f(w));

                let val = load_buff(xx as i32 + lx, yy as i32 + 1);
                col2 = col2.add(val.mult_f(w));
            }

            col = col.clamp01();
            col2 = col2.clamp01();

            // Scanline
            let wid = scanline_wid(col);
            let wid2 = scanline_wid(col2);
            let weights = scanline_weights(uv_ratio_y, wid);
            let weights2 = scanline_weights(1.0 - uv_ratio_y, wid2);
            let color = col.mult(weights).add(col2.mult(weights2));

            // Dotmask
            let mask_green_weight = 1.0 - (uv_ratio_x * 2.0 - 1.0).abs();
            let dot_mask_weights = lerp_color(
                Rgb::<f32>([1.0, 1.0 - CRT_DOT_MASK, 1.0]),
                Rgb::<f32>([1.0 - CRT_DOT_MASK, 1.0, 1.0 - CRT_DOT_MASK]),
                mask_green_weight,
            );
            let color = color.mult(dot_mask_weights).clamp01();

            let color = crt_inv_gamma(color);

            let p = color.clamp01();

            out.put_pixel(
                x,
                y,
                Rgba([
                    float_to_byte(p[0]),
                    float_to_byte(p[1]),
                    float_to_byte(p[2]),
                    255,
                ]),
            );
        }
    }
    out
}
