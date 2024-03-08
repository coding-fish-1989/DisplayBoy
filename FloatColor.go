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

package main

import (
	"image/color"
	"math"
)

// Three component color data with shader-like manipulation functions
type FloatColor struct {
	R float64
	G float64
	B float64
}

func (p FloatColor) Mult(c FloatColor) FloatColor {
	return FloatColor{p.R * c.R, p.G * c.G, p.B * c.B}
}

func (p FloatColor) MultF(f float64) FloatColor {
	return FloatColor{p.R * f, p.G * f, p.B * f}
}

func (p FloatColor) Div(c FloatColor) FloatColor {
	return FloatColor{p.R / c.R, p.G / c.G, p.B / c.B}
}

func (p FloatColor) DivF(f float64) FloatColor {
	return FloatColor{p.R / f, p.G / f, p.B / f}
}

func (p FloatColor) Add(c FloatColor) FloatColor {
	return FloatColor{p.R + c.R, p.G + c.G, p.B + c.B}
}

func (p FloatColor) AddF(f float64) FloatColor {
	return FloatColor{p.R + f, p.G + f, p.B + f}
}

func (p FloatColor) Sub(rhs FloatColor) FloatColor {
	return FloatColor{p.R - rhs.R, p.G - rhs.G, p.B - rhs.B}
}

func (p FloatColor) SubF(rhs float64) FloatColor {
	return FloatColor{p.R - rhs, p.G - rhs, p.B - rhs}
}

func (p FloatColor) Reciprocal() FloatColor {
	return FloatColor{1 / p.R, 1 / p.G, 1 / p.B}
}

func (p FloatColor) Clamp01() FloatColor {
	return FloatColor{clamp(p.R, 0, 1), clamp(p.G, 0, 1), clamp(p.B, 0, 1)}
}

func (p FloatColor) Ceil() FloatColor {
	return FloatColor{math.Ceil(p.R), math.Ceil(p.G), math.Ceil(p.B)}
}

func (p FloatColor) Floor() FloatColor {
	return FloatColor{math.Floor(p.R), math.Floor(p.G), math.Floor(p.B)}
}

func (p FloatColor) Pow(c FloatColor) FloatColor {
	return FloatColor{math.Pow(p.R, c.R), math.Pow(p.G, c.G), math.Pow(p.B, c.B)}
}

func (p FloatColor) PowF(e float64) FloatColor {
	return FloatColor{math.Pow(p.R, e), math.Pow(p.G, e), math.Pow(p.B, e)}
}

func (p FloatColor) Pow4() FloatColor {
	return FloatColor{p.R * p.R * p.R * p.R, p.G * p.G * p.G * p.G, p.B * p.B * p.B * p.B}
}

func (p FloatColor) Sqrt() FloatColor {
	return FloatColor{math.Sqrt(p.R), math.Sqrt(p.G), math.Sqrt(p.B)}
}

func (p FloatColor) Exp() FloatColor {
	return FloatColor{math.Exp(p.R), math.Exp(p.G), math.Exp(p.B)}
}

func (p FloatColor) Linear() FloatColor {
	return FloatColor{linear(p.R), linear(p.G), linear(p.B)}
}

func (p FloatColor) Gamma() FloatColor {
	return FloatColor{gamma(p.R), gamma(p.G), gamma(p.B)}
}

func (p FloatColor) Sin() FloatColor {
	return FloatColor{math.Sin(p.R), math.Sin(p.G), math.Sin(p.B)}
}

func (p FloatColor) Cos() FloatColor {
	return FloatColor{math.Cos(p.R), math.Cos(p.G), math.Cos(p.B)}
}

func (p FloatColor) Luminance() float64 {
	return p.R*0.2126 + p.G*0.7152 + p.B*0.0722
}

func (p FloatColor) NRGBA() color.NRGBA {
	return color.NRGBA{
		R: floatToByte(p.R),
		G: floatToByte(p.G),
		B: floatToByte(p.B),
		A: 255,
	}
}

func lerpColor(l, r FloatColor, t float64) FloatColor {
	return l.Add(r.Sub(l).MultF(t))
}

func makeFloatColor(v float64) FloatColor {
	return FloatColor{v, v, v}
}

func makeFloatColor3(r, g, b float64) FloatColor {
	return FloatColor{r, g, b}
}

func rgbaToLinearColor(r, g, b, a uint32) FloatColor {
	return FloatColor{linearTable[r>>8], linearTable[g>>8], linearTable[b>>8]}
}
