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

import "math"

func linear(v float64) float64 {
	if v <= 0.04045 {
		return v * (1.0 / 12.92)
	}
	return math.Pow((v+0.055)/1.055, 2.4)
}

func gamma(v float64) float64 {
	if v <= 0.0031308 {
		return v * 12.92
	}
	return 1.055*math.Pow(v, 1.0/2.4) - 0.055
}

func floatToByte(v float64) uint8 {
	if v >= 1 {
		return 255
	}
	return uint8(math.Floor(v * 256))
}

func clamp(f, low, high float64) float64 {
	return min(max(f, low), high)
}

func clamp01(f float64) float64 {
	return min(max(f, 0), 1)
}

func approximetlyEqual(a, b float64) bool {
	tolerance := 0.001
	diff := math.Abs(a - b)
	return diff < tolerance
}
