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

use image::{ImageBuffer, Luma, Rgb, Rgba};

// convert f32 gamma to linear
#[inline(always)]
pub fn to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        return v / 12.92;
    }
    return ((v + 0.055) / 1.055).powf(2.4);
}

// convert f32 linear to gamma
#[inline(always)]
pub fn to_gamma(v: f32) -> f32 {
    if v <= 0.0031308 {
        return v * 12.92;
    }
    return 1.055 * v.powf(1.0 / 2.4) - 0.055;
}

pub trait ShaderSupport {
    fn to_linear(&self) -> Self;
    fn to_linear_from_gamma(&self, gamma: f32) -> Self;
    fn to_gamma(&self) -> Self;
    fn add(&self, rhs: Self) -> Self;
    fn add_f(&self, rhs: f32) -> Self;
    fn sub(&self, rhs: Self) -> Self;
    fn sub_f(&self, rhs: f32) -> Self;
    fn mult(&self, rhs: Self) -> Self;
    fn mult_f(&self, rhs: f32) -> Self;
    fn div(&self, rhs: Self) -> Self;
    fn div_f(&self, rhs: f32) -> Self;
    fn pow(&self, rhs: Self) -> Self;
    fn pow_f(&self, rhs: f32) -> Self;
    fn exp(&self) -> Self;
    fn sqrt(&self) -> Self;
    fn clamp(&self, min: Self, max: Self) -> Self;
    fn clamp01(&self) -> Self;
    fn inv_sqrt(&self) -> Self;
    fn luminance(&self) -> f32;
}

impl ShaderSupport for Rgb<f32> {
    #[inline(always)]
    fn to_linear(&self) -> Self {
        Self([to_linear(self[0]), to_linear(self[1]), to_linear(self[2])])
    }

    #[inline(always)]
    fn to_linear_from_gamma(&self, gamma: f32) -> Self {
        Self([
            self[0].powf(gamma),
            self[1].powf(gamma),
            self[2].powf(gamma),
        ])
    }

    #[inline(always)]
    fn to_gamma(&self) -> Self {
        Self([to_gamma(self[0]), to_gamma(self[1]), to_gamma(self[2])])
    }

    #[inline(always)]
    fn add(&self, rhs: Self) -> Self {
        Self([self[0] + rhs[0], self[1] + rhs[1], self[2] + rhs[2]])
    }

    #[inline(always)]
    fn add_f(&self, rhs: f32) -> Self {
        Self([self[0] + rhs, self[1] + rhs, self[2] + rhs])
    }

    #[inline(always)]
    fn sub(&self, rhs: Self) -> Self {
        Self([self[0] - rhs[0], self[1] - rhs[1], self[2] - rhs[2]])
    }

    #[inline(always)]
    fn sub_f(&self, rhs: f32) -> Self {
        Self([self[0] - rhs, self[1] - rhs, self[2] - rhs])
    }

    #[inline(always)]
    fn mult(&self, rhs: Self) -> Self {
        Self([self[0] * rhs[0], self[1] * rhs[1], self[2] * rhs[2]])
    }

    #[inline(always)]
    fn mult_f(&self, rhs: f32) -> Self {
        Self([self[0] * rhs, self[1] * rhs, self[2] * rhs])
    }

    #[inline(always)]
    fn div(&self, rhs: Self) -> Self {
        Self([self[0] / rhs[0], self[1] / rhs[1], self[2] / rhs[2]])
    }

    #[inline(always)]
    fn div_f(&self, rhs: f32) -> Self {
        Self([self[0] / rhs, self[1] / rhs, self[2] / rhs])
    }

    #[inline(always)]
    fn pow(&self, rhs: Self) -> Self {
        Self([
            self[0].powf(rhs[0]),
            self[1].powf(rhs[1]),
            self[2].powf(rhs[2]),
        ])
    }

    #[inline(always)]
    fn pow_f(&self, rhs: f32) -> Self {
        Self([self[0].powf(rhs), self[1].powf(rhs), self[2].powf(rhs)])
    }

    #[inline(always)]
    fn exp(&self) -> Self {
        Self([
            fast_math::exp(self[0]),
            fast_math::exp(self[1]),
            fast_math::exp(self[2]),
        ])
    }

    #[inline(always)]
    fn sqrt(&self) -> Self {
        Self([self[0].sqrt(), self[1].sqrt(), self[2].sqrt()])
    }

    #[inline(always)]
    fn clamp(&self, min: Self, max: Self) -> Self {
        Self([
            self[0].max(min[0]).min(max[0]),
            self[1].max(min[1]).min(max[1]),
            self[2].max(min[2]).min(max[2]),
        ])
    }

    #[inline(always)]
    fn clamp01(&self) -> Self {
        Self([
            self[0].max(0.0).min(1.0),
            self[1].max(0.0).min(1.0),
            self[2].max(0.0).min(1.0),
        ])
    }

    #[inline(always)]
    fn inv_sqrt(&self) -> Self {
        Self([
            1.0 / self[0].sqrt(),
            1.0 / self[1].sqrt(),
            1.0 / self[2].sqrt(),
        ])
    }

    #[inline(always)]
    fn luminance(&self) -> f32 {
        0.2126 * self[0] + 0.7152 * self[1] + 0.0722 * self[2]
    }
}

#[inline(always)]
pub fn clamp(v: f32, min: f32, max: f32) -> f32 {
    v.max(min).min(max)
}

#[inline(always)]
pub fn float_to_byte(v: f32) -> u8 {
    if v >= 1.0 {
        return 255;
    }
    return (v * 256.0) as u8;
}

#[inline(always)]
pub fn lerp_color(a: Rgb<f32>, b: Rgb<f32>, t: f32) -> Rgb<f32> {
    Rgb::<f32>([
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ])
}

pub type FloatImage = ImageBuffer<Rgb<f32>, Vec<f32>>;
pub type AlphaImage = ImageBuffer<Luma<f32>, Vec<f32>>;

#[inline(always)]
pub fn rgba_u8_to_rgb_f32(rgb: Rgba<u8>) -> Rgb<f32> {
    Rgb::<f32>([
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
    ])
}
