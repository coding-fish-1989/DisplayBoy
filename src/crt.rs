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

use crate::shader_support;

use image::{Rgb, Rgba, RgbaImage};
use shader_support::*;

pub const CRT_MARGIN: u32 = 4;
pub const CRT_SCANLINE_WEIGHT: f32 = 0.3;
pub const CRT_LUM: f32 = 0.0;
pub const CRT_DOT_MASK: f32 = 0.05;
pub const CRT_GAMMA: f32 = 2.4;
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

pub fn crt(img: RgbaImage, src_scale: u32, scale: u32) -> RgbaImage {
    let load_buff = |x: i32, y: i32| -> Rgb<f32> {
        if y < 0 || y >= img.height() as i32 || x < 0 || x >= img.width() as i32 {
            return Rgb([0.0, 0.0, 0.0]);
        }
        let p = img.get_pixel(x as u32, y as u32);
        Rgb::<f32>([
            p[0] as f32 / 255.0,
            p[1] as f32 / 255.0,
            p[2] as f32 / 255.0,
        ])
    };

    let (src_width, mut src_height) = (img.width(), img.height());

    let mut top_margin = 0;
    if src_height < 240 && src_height >= 224 {
        top_margin = (240 - src_height) / 2;
        src_height = 240;
    }

    let scaled_margin = CRT_MARGIN * src_scale;
    top_margin += scaled_margin;
    let left_margin = scaled_margin;
    let src_height = src_height + scaled_margin * 2;
    let src_width = src_width + scaled_margin * 2;

    let (target_width, target_height) =
        calculate_scaled_buffer_size(src_width, src_height, src_scale);

    let mut buff = FloatImage::new(target_width, target_height);

    for y in 0..target_height {
        for x in 0..target_width {
            let src_x = (x * src_scale) as i32 - left_margin as i32;
            let src_y = (y * src_scale) as i32 - top_margin as i32;
            buff.put_pixel(x, y, load_buff(src_x, src_y).to_linear());
        }
    }

    let src_width = target_width;
    let src_height = target_height;

    let width = src_width * scale;
    let height = src_height * scale;

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

    let filter = 1.0 / scale as f32;

    // Precompute Lanczos weights
    let mut lanczos_weights = vec![vec![0.0; 6]; width as usize];
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
        let uv_ratio_y_orig = ratio_scale_y - yy;

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
            let mut uv_ratio_y = uv_ratio_y_orig;

            let wid = scanline_wid(col);
            let wid2 = scanline_wid(col2);

            let weights = scanline_weights(uv_ratio_y, wid);
            let weights2 = scanline_weights(1.0 - uv_ratio_y, wid2);

            uv_ratio_y = uv_ratio_y + 1.0 / 3.0 * filter as f32;
            let weights = weights.add(scanline_weights(uv_ratio_y, wid)).div_f(3.0);
            let weights2 = weights2.add(scanline_weights(1.0 - uv_ratio_y, wid2)).div_f(3.0);

            uv_ratio_y = uv_ratio_y - 2.0 / 3.0 * filter as f32;
            let weights = weights.add(scanline_weights(uv_ratio_y.abs(), wid).div_f(3.0));
            let weights2 = weights2.add(scanline_weights((1.0 - uv_ratio_y).abs(), wid2).div_f(3.0));

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
