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

use image::RgbaImage;
use shader_support::*;

pub struct ScaleInfo {
    pub scale_x: f32,
    pub scale_y: f32,
    pub respect_input_aspect_ratio: bool,
}

pub fn detect_src_scale(width: u32, height: u32) -> ScaleInfo {
    // Detect input resolution and infer the original image resolution
    let aspect_ratio = width as f32 / height as f32;
    if (aspect_ratio - (240.0 / 160.0)).abs() < 0.001 {
        // GBA
        let scale = (width as f32 / 240.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
        }
    } else if (aspect_ratio - (160.0 / 144.0)).abs() < 0.001 {
        // GBC
        let scale = (width as f32 / 160.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
        }
    } else if (aspect_ratio - (256.0 / 224.0)).abs() < 0.001
        || (aspect_ratio - (256.0 / 240.0)).abs() < 0.01
    {
        // Probably SNES or NES - raw resolution
        let scale = (width as f32 / 256.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
        }
    } else if (aspect_ratio - (252.0 / 238.0)).abs() < 0.001 {
        // Probably NES - raw resolution
        // Appears in spiritualized1997 NES video.json
        let scale = (width as f32 / 252.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
        }
    } else if (aspect_ratio - (240.0 / 224.0)).abs() < 0.001
        || (aspect_ratio - (240.0 / 238.0)).abs() < 0.001
    {
        // Probably NES - raw resolution
        // Appears in spiritualized1997 NES video.json
        let scale = (width as f32 / 240.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
        }
    } else if (aspect_ratio - (64.0 / 49.0)).abs() < 0.001
        || (aspect_ratio - (8.0 / 7.0)).abs() < 0.001
    {
        // Probably upscaled CRT with height of 224
        ScaleInfo {
            scale_x: width as f32 / 256.0,
            scale_y: height as f32 / 224.0,
            respect_input_aspect_ratio: true,
        }
    } else if (aspect_ratio - (128.0 / 105.0)).abs() < 0.001
        || (aspect_ratio - (16.0 / 15.0)).abs() < 0.001
    {
        // Probably upscaled CRT with height of 240
        ScaleInfo {
            scale_x: width as f32 / 256.0,
            scale_y: height as f32 / 240.0,
            respect_input_aspect_ratio: true,
        }
    } else if width == 512 && (height == 240 || height == 224) {
        // agg23's SNES core support
        // https://github.com/agg23/openfpga-SNES/blob/master/dist/Cores/agg23.SNES/video.json
        ScaleInfo {
            scale_x: 2.0,
            scale_y: 1.0,
            respect_input_aspect_ratio: false,
        }
    } else {
        // Fallback to 240p if too large
        let scale = (height as f32 / 240.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
        }
    }
}

#[inline(always)]
fn conservative_ceil_to_u32(v: f32) -> u32 {
    // Drop the fractional part if it's nearly zero.
    // This will allow for non integer original image scaling to be scaled back to the original resolution better.
    if v.fract().abs() < 0.01 {
        return v as u32;
    }
    v.ceil() as u32
}

#[inline(always)]
pub fn calculate_scaled_buffer_size(width: u32, height: u32, scale: &ScaleInfo) -> (u32, u32) {
    let width = conservative_ceil_to_u32(width as f32 / scale.scale_x as f32);
    let height = conservative_ceil_to_u32(height as f32 / scale.scale_y as f32);
    (width, height)
}

pub fn prepare_src_image(img: &RgbaImage, src_scale: &ScaleInfo) -> FloatImage {
    let src_width = img.width();
    let src_height = img.height();

    let (target_width, target_height) =
        calculate_scaled_buffer_size(src_width, src_height, src_scale);
    let mut buff = FloatImage::new(target_width, target_height);
    let x_target_half_texel = 1.0 / (target_width as f32 * 2.0);
    let y_target_half_texel = 1.0 / (target_height as f32 * 2.0);
    for y in 0..target_height {
        for x in 0..target_width {
            let x_coord = x as f32 / target_width as f32 + x_target_half_texel;
            let y_coord = y as f32 / target_height as f32 + y_target_half_texel;
            let x_src = (x_coord * src_width as f32).floor() as u32;
            let y_src = (y_coord * src_height as f32).floor() as u32;
            let p = img.get_pixel(x_src, y_src);
            let p = rgba_u8_to_rgb_f32(*p).to_linear();
            buff.put_pixel(x, y, p);
        }
    }
    buff
}
