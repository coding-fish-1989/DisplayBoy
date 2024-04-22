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

use crate::{
    shader_support::{self, lerp_color, rgba_u8_to_rgb_f32, FloatImage, ShaderSupport},
    utils,
};

use image::{GenericImageView, Rgb, RgbaImage};
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

pub struct ScaleInfo {
    pub scale_x: f32,
    pub scale_y: f32,
    pub respect_input_aspect_ratio: bool,
    pub detected: bool,
    pub bilinear_recommended: bool,
    pub device_name: String,
}

#[wasm_bindgen(js_name = getSourceDeviceName)]
pub fn get_source_device_name(width: u32, height: u32) -> String {
    set_panic_hook();

    detect_src_scale(width, height, 240).device_name
}

pub fn detect_src_scale(width: u32, height: u32, fallback_height: u32) -> ScaleInfo {
    // Detect input resolution and infer the original image resolution
    let aspect_ratio = width as f32 / height as f32;
    if (aspect_ratio - (240.0 / 160.0)).abs() < 0.001 {
        // GBA
        let scale = (width as f32 / 240.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
            detected: true,
            bilinear_recommended: false,
            device_name: "GBA".to_string(),
        }
    } else if (aspect_ratio - (160.0 / 144.0)).abs() < 0.001 {
        // GB/GBC
        let scale = (width as f32 / 160.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
            detected: true,
            bilinear_recommended: false,
            device_name: "GB/GBC".to_string(),
        }
    } else if (aspect_ratio - (256.0 / 224.0)).abs() < 0.001
        || (aspect_ratio - (256.0 / 240.0)).abs() < 0.01
    {
        // This is a bit confusing, but GB Camera has a resolution of 128 x 112,
        // which is exactly the half of some NES/SNES resolution.
        // It doesn't actually make any different in terms of how scales are calculated,
        // but it is detected and labeled as such to not confuse the user.
        let is_gb_camera = width == 128 && height == 112;
        let scale = (width as f32 / 256.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
            detected: true,
            bilinear_recommended: false,
            device_name: if is_gb_camera {
                "GB Camera"
            } else {
                "NES/SNES"
            }
            .to_string(),
        }
    } else if (aspect_ratio - (252.0 / 238.0)).abs() < 0.001 {
        // Probably NES - raw resolution
        // Appears in spiritualized1997 NES video.json
        let scale = (width as f32 / 252.0).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
            detected: true,
            bilinear_recommended: false,
            device_name: "NES".to_string(),
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
            detected: true,
            bilinear_recommended: false,
            device_name: "NES".to_string(),
        }
    } else if (aspect_ratio - (64.0 / 49.0)).abs() < 0.001
        || (aspect_ratio - (8.0 / 7.0)).abs() < 0.001
    {
        // Probably upscaled CRT with height of 224
        ScaleInfo {
            scale_x: width as f32 / 256.0,
            scale_y: height as f32 / 224.0,
            respect_input_aspect_ratio: true,
            detected: true,
            bilinear_recommended: false,
            device_name: "CRT (224) - PAR (8:7)".to_string(),
        }
    } else if (aspect_ratio - (128.0 / 105.0)).abs() < 0.001
        || (aspect_ratio - (16.0 / 15.0)).abs() < 0.001
    {
        // Probably upscaled CRT with height of 240
        ScaleInfo {
            scale_x: width as f32 / 256.0,
            scale_y: height as f32 / 240.0,
            respect_input_aspect_ratio: true,
            detected: true,
            bilinear_recommended: false,
            device_name: "CRT (240) - PAR (8:7)".to_string(),
        }
    } else if width == 512 && (height == 240 || height == 224) {
        // agg23's SNES core support
        // https://github.com/agg23/openfpga-SNES/blob/master/dist/Cores/agg23.SNES/video.json
        ScaleInfo {
            scale_x: 2.0,
            scale_y: 1.0,
            respect_input_aspect_ratio: false,
            detected: true,
            bilinear_recommended: false,
            device_name: "SNES (agg23)".to_string(),
        }
    } else {
        let scale = (height as f32 / fallback_height as f32).max(1.0);
        ScaleInfo {
            scale_x: scale,
            scale_y: scale,
            respect_input_aspect_ratio: false,
            detected: false,
            bilinear_recommended: scale > 1.0,
            device_name: "Unknown".to_string(),
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

pub fn exif_orientation_dimension(width: u32, height: u32, orientation: u32) -> (u32, u32) {
    match orientation {
        1 => (width, height),
        2 => (width, height),
        3 => (width, height),
        4 => (width, height),
        5 => (height, width),
        6 => (height, width),
        7 => (height, width),
        8 => (height, width),
        _ => (width, height),
    }
}

#[inline(always)]
pub fn exif_orientation_transform_coordinate(
    width: u32,
    height: u32,
    orientation: u32,
    x: i32,
    y: i32,
) -> (i32, i32) {
    let (width, height) = (width as i32, height as i32);
    match orientation {
        1 => (x, y),
        2 => (width - x - 1, y),
        3 => (width - x - 1, height - y - 1),
        4 => (x, height - y - 1),
        5 => (y, x),
        6 => (y, width - x - 1),
        7 => (height - y - 1, width - x - 1),
        8 => (height - y - 1, x),
        _ => (x, y),
    }
}

pub fn downsample_image_nearest_neighbour(
    src: &RgbaImage,
    scale: &ScaleInfo,
    output_gamma: bool,
    exif_orientation: u32,
) -> FloatImage {
    let (src_width, src_height) =
        exif_orientation_dimension(src.width(), src.height(), exif_orientation);
    let (dst_width, dst_height) = calculate_scaled_buffer_size(src_width, src_height, scale);

    // Downsample while applying EXIF orientation
    let mut out = FloatImage::from_fn(dst_width, dst_height, |x, y| {
        let x_coord = (x as f32 + 0.5) / dst_width as f32;
        let y_coord = (y as f32 + 0.5) / dst_height as f32;
        let src_x = (x_coord * src_width as f32) as u32;
        let src_y = (y_coord * src_height as f32) as u32;
        let (src_x, src_y) = exif_orientation_transform_coordinate(
            src_width,
            src_height,
            exif_orientation,
            src_x as i32,
            src_y as i32,
        );
        rgba_u8_to_rgb_f32(*src.get_pixel(src_x as u32, src_y as u32))
    });

    if output_gamma {
        out
    } else {
        // Convert to linear if needed
        out.enumerate_pixels_mut().for_each(|(_x, _y, pixel)| {
            *pixel = pixel.to_linear();
        });
        out
    }
}

pub fn downsample_image_bilinear(
    src: &RgbaImage,
    scale: &ScaleInfo,
    output_gamma: bool,
    exif_orientation: u32,
) -> FloatImage {
    let (src_width, src_height) =
        exif_orientation_dimension(src.width(), src.height(), exif_orientation);
    let (final_dst_width, final_dst_height) =
        calculate_scaled_buffer_size(src_width, src_height, scale);

    // Precompute gamma (u8) to linear (f32) conversion table
    let mut gamma_to_linear = [0.0; 256];
    for i in 0..256 {
        gamma_to_linear[i] = shader_support::to_linear(i as f32 / 255.0);
    }

    // Convert gamma to linear while applying EXIF orientation
    // Bilinear interpolation is done in linear space for better quality.
    let mut src = FloatImage::from_fn(src_width, src_height, |x, y| unsafe {
        let (x, y) = exif_orientation_transform_coordinate(
            src_width,
            src_height,
            exif_orientation,
            x as i32,
            y as i32,
        );
        let pixel = src.unsafe_get_pixel(x as u32, y as u32);
        let r = gamma_to_linear[pixel[0] as usize];
        let g = gamma_to_linear[pixel[1] as usize];
        let b = gamma_to_linear[pixel[2] as usize];
        Rgb::<f32>([r, g, b])
    });

    // Downsample the image using bilinear interpolation to the final resolution.
    // This is done in multiple passes to avoid aliasing artifacts.
    while src.width() > final_dst_width {
        let (src_width, src_height) = (src.width(), src.height());
        let (dst_width, dst_height) = if src_width as f32 / 2.0 > final_dst_width as f32 {
            (
                conservative_ceil_to_u32(src_width as f32 / 2.0),
                conservative_ceil_to_u32(src_height as f32 / 2.0),
            )
        } else {
            (final_dst_width, final_dst_height)
        };

        if dst_width * 2 == src_width && dst_height * 2 == src_height {
            // Exact half downsample fast path
            src = FloatImage::from_fn(dst_width, dst_height, |x, y| unsafe {
                let (src_x, src_y) = (x * 2, y * 2);
                let p00 = src.unsafe_get_pixel(src_x, src_y);
                let p01 = src.unsafe_get_pixel(src_x + 1, src_y);
                let p10 = src.unsafe_get_pixel(src_x, src_y + 1);
                let p11 = src.unsafe_get_pixel(src_x + 1, src_y + 1);
                p00.add(p01).add(p10).add(p11).mult_f(0.25)
            });
        } else {
            let width_max = src_width - 1;
            let height_max = src_height - 1;
            let (scale_x, scale_y) = (
                src_width as f32 / dst_width as f32,
                src_height as f32 / dst_height as f32,
            );
            src = FloatImage::from_fn(dst_width, dst_height, |x, y| unsafe {
                let xf = ((x as f32 + 0.5) * scale_x - 0.5).max(0.0);
                let yf = ((y as f32 + 0.5) * scale_y - 0.5).max(0.0);

                let src_x = xf as u32;
                let src_y = yf as u32;
                let x_weight = xf - src_x as f32;
                let y_weight = yf - src_y as f32;

                let src_x_next = (src_x + 1).min(width_max);
                let src_y_next = (src_y + 1).min(height_max);

                let p00 = src.unsafe_get_pixel(src_x, src_y);
                let p01 = src.unsafe_get_pixel(src_x_next, src_y);
                let p10 = src.unsafe_get_pixel(src_x, src_y_next);
                let p11 = src.unsafe_get_pixel(src_x_next, src_y_next);
                let p0 = lerp_color(p00, p01, x_weight);
                let p1 = lerp_color(p10, p11, x_weight);
                lerp_color(p0, p1, y_weight)
            });
        }
    }

    // Back to gamma if needed
    if output_gamma {
        src.enumerate_pixels_mut().for_each(|(_x, _y, pixel)| {
            *pixel = pixel.to_gamma();
        });
    }

    src
}
