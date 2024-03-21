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

use image::{Rgb, Rgba, RgbaImage};
use shader_support::*;
use scaling::*;

pub struct DisplayProfile {
    pub r: f32,
    pub gr: f32,
    pub br: f32,
    pub rg: f32,
    pub g: f32,
    pub bg: f32,
    pub rb: f32,
    pub gb: f32,
    pub b: f32,
    pub lum: f32,
    pub gamma: f32,
    pub gamma_offset: f32,
    pub bgr: bool,
}

#[inline(always)]
fn int_smear_func(z: f32, coeffs: &[f32; 7]) -> f32 {
    let z2 = z * z;
    let mut zn = z;
    let mut ret = 0.0;
    for i in 0..7 {
        ret += zn * coeffs[i];
        zn *= z2;
    }
    ret
}

#[inline(always)]
fn int_smear(x: f32, dx: f32, d: f32, coeffs: &[f32; 7]) -> f32 {
    let zl = clamp((x - dx * 0.5) / d, -1.0, 1.0);
    let zh = clamp((x + dx * 0.5) / d, -1.0, 1.0);
    d * (int_smear_func(zh, coeffs) - int_smear_func(zl, coeffs)) / dx
}

#[inline(always)]
fn color_correct(c: Rgb<f32>, p: &DisplayProfile) -> Rgb<f32> {
    let gamma = p.gamma + p.gamma_offset;
    let c = c.pow_f(gamma / p.gamma);
    let c = c.mult_f(p.lum);
    let c = c.clamp01();
    let r = (p.r * c[0]) + (p.gr * c[1]) + (p.br * c[2]);
    let g = (p.rg * c[0]) + (p.g * c[1]) + (p.bg * c[2]);
    let b = (p.rb * c[0]) + (p.gb * c[1]) + (p.b * c[2]);
    let c = Rgb::<f32>([r, g, b]);
    c.clamp01()
}

pub fn color_gb(
    img: RgbaImage,
    src_scale: ScaleInfo,
    scale: u32,
    lcd_mode: u32,
    prof: &DisplayProfile,
) -> RgbaImage {
	let src_img = prepare_src_image(&img, &src_scale);

    let src_width = src_img.width();
    let src_height = src_img.height();
    let width = src_width * scale;
    let height = src_height * scale;

    let src_width_f = src_width as f32;
    let src_height_f = src_height as f32;

    let point_sample_buff = |x: f32, y: f32| -> Rgb<f32> {
        let x = ((x * src_width_f).floor() as i32)
            .min(src_width as i32 - 1)
            .max(0) as u32;
        let y = ((y * src_height_f).floor() as i32)
            .min(src_height as i32 - 1)
            .max(0) as u32;
        *src_img.get_pixel(x, y)
    };

    let load_buff = |x: i32, y: i32| -> Rgb<f32> {
        *src_img.get_pixel(
            x.min(src_width as i32 - 1).max(0) as u32,
            y.min(src_height as i32 - 1).max(0) as u32,
        )
    };

    let texel_size_x = 1.0 / src_width_f;
    let texel_size_y = 1.0 / src_height_f;
    let out_texel_size_x = 1.0 / width as f32;
    let out_texel_size_y = 1.0 / height as f32;

    let coeffs_x = [
        1.0,
        -2.0 / 3.0,
        -1.0 / 5.0,
        4.0 / 7.0,
        -1.0 / 9.0,
        -2.0 / 11.0,
        1.0 / 13.0,
    ];
    let coeffs_y = [
        1.0,
        0.0,
        -4.0 / 5.0,
        2.0 / 7.0,
        4.0 / 9.0,
        -4.0 / 11.0,
        1.0 / 13.0,
    ];

    let color_low = 0.8;
    let color_high = 1.0;
    let scanline_depth = 0.1;

    let mut out = RgbaImage::new(width as u32, height as u32);

    for y in (0..height).step_by(1) {
        let tex_coord_y = out_texel_size_y * (y as f32 + 0.5);
        for x in (0..width).step_by(1) {
            let tex_coord_x = out_texel_size_x * (x as f32 + 0.5);

            let mut p: Rgb<f32>;

            if lcd_mode == 1 {
                let tli_x = (tex_coord_x * src_width_f - 0.4999) as i32;
                let tli_y = (tex_coord_y * src_height_f - 0.4999) as i32;

                let subpix = (tex_coord_x * src_width_f - 0.4999 - tli_x as f32) * 3.0;
                let rsubpix = out_texel_size_x * src_width_f * 3.0;

                let mut lcol = Rgb::<f32>([
                    int_smear(subpix + 1.0, rsubpix, 1.5, &coeffs_x),
                    int_smear(subpix, rsubpix, 1.5, &coeffs_x),
                    int_smear(subpix - 1.0, rsubpix, 1.5, &coeffs_x),
                ]);
                let mut rcol = Rgb::<f32>([
                    int_smear(subpix - 2.0, rsubpix, 1.5, &coeffs_x),
                    int_smear(subpix - 3.0, rsubpix, 1.5, &coeffs_x),
                    int_smear(subpix - 4.0, rsubpix, 1.5, &coeffs_x),
                ]);

                if prof.bgr {
                    let r = lcol[0];
                    let g = lcol[1];
                    let b = lcol[2];
                    lcol[0] = b;
                    lcol[1] = g;
                    lcol[2] = r;
                    let r = rcol[0];
                    let g = rcol[1];
                    let b = rcol[2];
                    rcol[0] = b;
                    rcol[1] = g;
                    rcol[2] = r;
                }

                let subpix = tex_coord_y * src_height_f - 0.4999 - tli_y as f32;
                let rsubpix = out_texel_size_y * src_height_f;
                let tcol = int_smear(subpix, rsubpix, 0.63, &coeffs_y);
                let bcol = int_smear(subpix - 1.0, rsubpix, 0.63, &coeffs_y);

                let top_left_color = load_buff(tli_x, tli_y).mult(lcol).mult_f(tcol);

                let bottom_right_color = load_buff(tli_x + 1, tli_y + 1).mult(rcol).mult_f(bcol);
                let bottom_left_color = load_buff(tli_x, tli_y + 1).mult(lcol).mult_f(bcol);
                let top_right_color = load_buff(tli_x + 1, tli_y).mult(rcol).mult_f(tcol);

                p = top_left_color
                    .add(bottom_right_color)
                    .add(bottom_left_color)
                    .add(top_right_color)
            } else if lcd_mode == 0 {
                /*
                    Expat License

                    Copyright (c) 2015-2024 Lior Halphon

                    Permission is hereby granted, free of charge, to any person obtaining a copy
                    of this software and associated documentation files (the "Software"), to deal
                    in the Software without restriction, including without limitation the rights
                    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
                    copies of the Software, and to permit persons to whom the Software is
                    furnished to do so, subject to the following conditions:

                    The above copyright notice and this permission notice shall be included in all
                    copies or substantial portions of the Software.

                    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
                    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
                    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
                    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
                    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
                    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
                    SOFTWARE.
                */
                let sub_pos_x = (tex_coord_x * src_width_f).fract() * 6.0;
                let sub_pos_y = (tex_coord_y * src_height_f).fract() * 6.0;

                let mut center = point_sample_buff(tex_coord_x, tex_coord_y);
                let mut left = point_sample_buff(tex_coord_x - texel_size_x, tex_coord_y);
                let mut right = point_sample_buff(tex_coord_x + texel_size_x, tex_coord_y);

                if sub_pos_y < 1.0 {
                    let top_n = point_sample_buff(tex_coord_x, tex_coord_y - texel_size_y);
                    let left_n =
                        point_sample_buff(tex_coord_x - texel_size_x, tex_coord_y - texel_size_y);
                    let right_n =
                        point_sample_buff(tex_coord_x + texel_size_x, tex_coord_y - texel_size_y);
                    center = lerp_color(center, top_n, 0.5 - sub_pos_y * 0.5);
                    left = lerp_color(left, left_n, 0.5 - sub_pos_y * 0.5);
                    right = lerp_color(right, right_n, 0.5 - sub_pos_y * 0.5);
                    center = center.mult_f(sub_pos_y * scanline_depth + (1.0 - scanline_depth));
                    left = left.mult_f(sub_pos_y * scanline_depth + (1.0 - scanline_depth));
                    right = right.mult_f(sub_pos_y * scanline_depth + (1.0 - scanline_depth));
                } else if sub_pos_y > 5.0 {
                    let bottom_n = point_sample_buff(tex_coord_x, tex_coord_y + texel_size_y);
                    let left_n =
                        point_sample_buff(tex_coord_x - texel_size_x, tex_coord_y + texel_size_y);
                    let right_n =
                        point_sample_buff(tex_coord_x + texel_size_x, tex_coord_y + texel_size_y);
                    center = lerp_color(center, bottom_n, (sub_pos_y - 5.0) * 0.5);
                    left = lerp_color(left, left_n, (sub_pos_y - 5.0) * 0.5);
                    right = lerp_color(right, right_n, (sub_pos_y - 5.0) * 0.5);
                    center =
                        center.mult_f((6.0 - sub_pos_y) * scanline_depth + (1.0 - scanline_depth));
                    left = left.mult_f((6.0 - sub_pos_y) * scanline_depth + (1.0 - scanline_depth));
                    right =
                        right.mult_f((6.0 - sub_pos_y) * scanline_depth + (1.0 - scanline_depth));
                }

                let mid_left = lerp_color(left, center, 0.5);
                let mid_right = lerp_color(right, center, 0.5);

                if sub_pos_x < 1.0 {
                    p = lerp_color(
                        Rgb::<f32>([
                            color_high * center[0],
                            color_low * center[1],
                            color_high * left[2],
                        ]),
                        Rgb::<f32>([
                            color_high * center[0],
                            color_low * center[1],
                            color_low * left[2],
                        ]),
                        sub_pos_x,
                    );
                } else if sub_pos_x < 2.0 {
                    p = lerp_color(
                        Rgb::<f32>([
                            color_high * center[0],
                            color_low * center[1],
                            color_low * left[2],
                        ]),
                        Rgb::<f32>([
                            color_high * center[0],
                            color_high * center[1],
                            color_low * mid_left[2],
                        ]),
                        sub_pos_x - 1.0,
                    );
                } else if sub_pos_x < 3.0 {
                    p = lerp_color(
                        Rgb::<f32>([
                            color_high * center[0],
                            color_high * center[1],
                            color_low * mid_left[2],
                        ]),
                        Rgb::<f32>([
                            color_low * mid_right[0],
                            color_high * center[1],
                            color_low * center[2],
                        ]),
                        sub_pos_x - 2.0,
                    );
                } else if sub_pos_x < 4.0 {
                    p = lerp_color(
                        Rgb::<f32>([
                            color_low * mid_right[0],
                            color_high * center[1],
                            color_low * center[2],
                        ]),
                        Rgb::<f32>([
                            color_low * right[0],
                            color_high * center[1],
                            color_high * center[2],
                        ]),
                        sub_pos_x - 3.0,
                    );
                } else if sub_pos_x < 5.0 {
                    p = lerp_color(
                        Rgb::<f32>([
                            color_low * right[0],
                            color_high * center[1],
                            color_high * center[2],
                        ]),
                        Rgb::<f32>([
                            color_low * right[0],
                            color_low * mid_right[1],
                            color_high * center[2],
                        ]),
                        sub_pos_x - 4.0,
                    );
                } else {
                    p = lerp_color(
                        Rgb::<f32>([
                            color_low * right[0],
                            color_low * mid_right[1],
                            color_high * center[2],
                        ]),
                        Rgb::<f32>([
                            color_high * right[0],
                            color_low * right[1],
                            color_high * center[2],
                        ]),
                        sub_pos_x - 5.0,
                    );
                }
            } else {
                p = point_sample_buff(tex_coord_x, tex_coord_y);
            }

            p = p.clamp01();
            p = color_correct(p, &prof);
            p = p.to_gamma();
            out.put_pixel(
                x as u32,
                y as u32,
                Rgba::<u8>([
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
