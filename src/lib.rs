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

mod crt;
mod gb;
mod gbc;
mod scaling;
mod shader_support;
mod utils;

use base64::{engine::general_purpose, Engine as _};
use exif::*;
use scaling::*;
use std::io::Cursor;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

fn parse_exif_orientation(data: &Vec<u8>) -> u32 {
    let exifreader = exif::Reader::new();
    let exif_result = exifreader.read_from_container(&mut Cursor::new(&data));
    if exif_result.is_err() {
        return 1;
    }
    match exif_result
        .unwrap()
        .get_field(Tag::Orientation, In::PRIMARY)
    {
        Some(orientation) => match orientation.value.get_uint(0) {
            Some(v @ 1..=8) => v,
            _ => 1,
        },
        None => 1,
    }
}

#[wasm_bindgen(js_name = processImageGb)]
pub fn process_image_gb(
    mode: i32,
    dither: bool,
    brightness: f32,
    contrast: f32,
    invert: bool,
    edge_enhancement_level: f32,
    height_cap: i32,
    data: Vec<u8>,
) -> String {
    set_panic_hook();

    let exif_orientation = parse_exif_orientation(&data);
    let img = image::load_from_memory(&data).unwrap();
    let (width, height) = exif_orientation_dimension(img.width(), img.height(), exif_orientation);

    let gb = gb::GbDisplayProfile {
        foreground_r: 19.0 / 255.0,
        foreground_g: 74.0 / 255.0,
        foreground_b: 7.0 / 255.0,
        foreground_a: 1.0,
        background_r: 170.0 / 255.0,
        background_g: 181.0 / 255.0,
        background_b: 19.0 / 255.0,
    };

    let gbp = gb::GbDisplayProfile {
        foreground_r: 0.0 / 255.0,
        foreground_g: 0.0 / 255.0,
        foreground_b: 0.0 / 255.0,
        foreground_a: 1.0,
        background_r: 164.0 / 255.0,
        background_g: 169.0 / 255.0,
        background_b: 137.0 / 255.0,
    };

    let gbl = gb::GbDisplayProfile {
        foreground_r: 0.0 / 255.0,
        foreground_g: 46.0 / 255.0,
        foreground_b: 44.0 / 255.0,
        foreground_a: 1.0,
        background_r: 0.0 / 255.0,
        background_g: 181.0 / 255.0,
        background_b: 176.0 / 255.0,
    };

    let prof = match mode {
        0 => gb,
        1 => gbp,
        2 => gbl,
        _ => gb,
    };

    let adjustment = gb::GbColorAdjustment {
        dither: dither,
        brightness: brightness,
        contrast: contrast,
        invert: invert,
        edge_enhancement_level: edge_enhancement_level,
    };

    let src_scale = detect_src_scale(
        width,
        height,
        if height_cap < 0 {
            144
        } else {
            height_cap as u32
        },
    );
    let result = gb::gb_mono(
        img.into_rgba8(),
        exif_orientation,
        src_scale,
        &prof,
        &adjustment,
    );

    let mut buf = Vec::new();
    let _ = result.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    return general_purpose::STANDARD.encode(&buf);
}

#[wasm_bindgen(js_name = processImageGbCustom)]
pub fn process_image_gb_custom(
    fg_color: String,
    fg_alpha: i32,
    bg_color: String,
    dither: bool,
    brightness: f32,
    contrast: f32,
    invert: bool,
    edge_enhancement_level: f32,
    height_cap: i32,
    data: Vec<u8>,
) -> String {
    set_panic_hook();

    let img = image::load_from_memory(&data).unwrap();
    let exif_orientation = parse_exif_orientation(&data);
    let (width, height) = exif_orientation_dimension(img.width(), img.height(), exif_orientation);

    // Color input is in the format #RRGGBB
    let prof = gb::GbDisplayProfile {
        foreground_r: i32::from_str_radix(&fg_color[1..3], 16).unwrap() as f32 / 255.0,
        foreground_g: i32::from_str_radix(&fg_color[3..5], 16).unwrap() as f32 / 255.0,
        foreground_b: i32::from_str_radix(&fg_color[5..7], 16).unwrap() as f32 / 255.0,
        foreground_a: fg_alpha as f32 / 100.0,
        background_r: i32::from_str_radix(&bg_color[1..3], 16).unwrap() as f32 / 255.0,
        background_g: i32::from_str_radix(&bg_color[3..5], 16).unwrap() as f32 / 255.0,
        background_b: i32::from_str_radix(&bg_color[5..7], 16).unwrap() as f32 / 255.0,
    };

    let adjustment = gb::GbColorAdjustment {
        dither: dither,
        brightness: brightness,
        contrast: contrast,
        invert: invert,
        edge_enhancement_level: edge_enhancement_level,
    };

    let src_scale = detect_src_scale(
        width,
        height,
        if height_cap < 0 {
            144
        } else {
            height_cap as u32
        },
    );
    let result = gb::gb_mono(
        img.into_rgba8(),
        exif_orientation,
        src_scale,
        &prof,
        &adjustment,
    );

    let mut buf = Vec::new();
    let _ = result.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    return general_purpose::STANDARD.encode(&buf);
}

#[wasm_bindgen(js_name = processImageGbc)]
pub fn process_image_gbc(
    scale: u32,
    lcd_mode: u32,
    color_mode: u32,
    height_cap: i32,
    data: Vec<u8>,
) -> String {
    set_panic_hook();

    // Pokefan531's GBC Display Profile
    let gbc = gbc::DisplayProfile {
        r: 0.80,
        gr: 0.275,
        br: -0.075,
        rg: 0.135,
        g: 0.64,
        bg: 0.225,
        rb: 0.195,
        gb: 0.155,
        b: 0.65,
        lum: 0.93,
        gamma: 2.2,
        gamma_offset: -0.5,
        bgr: false,
    };

    let gba = gbc::DisplayProfile {
        r: 0.80,
        gr: 0.275,
        br: -0.075,
        rg: 0.135,
        g: 0.64,
        bg: 0.225,
        rb: 0.195,
        gb: 0.155,
        b: 0.65,
        lum: 0.93,
        gamma: 2.0,
        gamma_offset: 0.5,
        bgr: true,
    };

    let gba_sp = gbc::DisplayProfile {
        r: 0.86,
        gr: 0.10,
        br: -0.06,
        rg: 0.03,
        g: 0.745,
        bg: 0.0675,
        rb: 0.0025,
        gb: -0.03,
        b: 1.0275,
        lum: 0.97,
        gamma: 2.0,
        gamma_offset: 0.0,
        bgr: false,
    };

    let gba_sp_white = gbc::DisplayProfile {
        r: 0.955,
        gr: 0.11,
        br: -0.065,
        rg: 0.0375,
        g: 0.885,
        bg: 0.0775,
        rb: 0.0025,
        gb: -0.03,
        b: 1.0275,
        lum: 0.94,
        gamma: 2.0,
        gamma_offset: 0.0,
        bgr: false,
    };

    let prof = match color_mode {
        0 => gbc,
        1 => gba,
        2 => gba_sp,
        3 => gba_sp_white,
        _ => gbc,
    };

    let fallback_height = if height_cap < 0 {
        match color_mode {
            0 => 144,
            1 => 160,
            2 => 160,
            3 => 160,
            _ => 144,
        }
    } else {
        height_cap as u32
    };

    let exif_orientation = parse_exif_orientation(&data);
    let img = image::load_from_memory(&data).unwrap();
    let (width, height) = exif_orientation_dimension(img.width(), img.height(), exif_orientation);

    let src_scale = detect_src_scale(width, height, fallback_height);
    let result = gbc::color_gb(
        img.into_rgba8(),
        exif_orientation,
        src_scale,
        scale,
        lcd_mode,
        &prof,
    );

    let mut buf = Vec::new();
    let _ = result.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    return general_purpose::STANDARD.encode(&buf);
}

#[wasm_bindgen(js_name = processImageCrt)]
pub fn process_image_crt(
    scale: u32,
    explicit_aspect_ratio: bool,
    pixel_aspect_ratio: f32,
    height_cap: i32,
    data: Vec<u8>,
) -> String {
    set_panic_hook();

    let exif_orientation = parse_exif_orientation(&data);
    let img = image::load_from_memory(&data).unwrap();
    let (width, height) = exif_orientation_dimension(img.width(), img.height(), exif_orientation);

    let src_scale = detect_src_scale(
        width,
        height,
        if height_cap < 0 {
            240
        } else {
            height_cap as u32
        },
    );
    let result = crt::crt(
        img.into_rgba8(),
        exif_orientation,
        src_scale,
        scale,
        explicit_aspect_ratio,
        pixel_aspect_ratio,
    );

    let mut buf = Vec::new();
    let _ = result.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    return general_purpose::STANDARD.encode(&buf);
}
