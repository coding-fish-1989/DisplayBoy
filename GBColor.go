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
	"image"
	"math"
)

type DisplayProfile struct {
	R, GR, BR, RG, G, BG, RB, GB, B, Lum float64
	Gamma, GammaOffset                   float64
	BGR                                  bool
}

func colorCorrect(c FloatColor, p DisplayProfile) FloatColor {
	// Compress to profile gamma, offset and bring it back to linear
	c = c.PowF((p.Gamma + p.GammaOffset) / p.Gamma)
	// Apply matrix in linear space with adjusted gamma
	c = c.MultF(p.Lum)
	c = c.Clamp01()
	// Apply color matrix
	c.R = (p.R * c.R) + (p.GR * c.G) + (p.BR * c.B)
	c.G = (p.RG * c.R) + (p.G * c.G) + (p.BG * c.B)
	c.B = (p.RB * c.R) + (p.GB * c.G) + (p.B * c.B)
	c = c.Clamp01()
	return c
}

func calculateScaledBufferSize(width, height, scale int) (int, int) {
	return int(math.Ceil(float64(width) / float64(scale))), int(math.Ceil(float64(height) / float64(scale)))
}

func intSmearFunc(z float64, coeffs [7]float64) float64 {
	z2 := z * z
	zn := z
	ret := 0.0
	for i := 0; i < 7; i++ {
		ret += zn * coeffs[i]
		zn *= z2
	}
	return ret
}

func intSmear(x, dx, d float64, coeffs [7]float64) float64 {
	zl := clamp((x-dx*0.5)/d, -1.0, 1.0)
	zh := clamp((x+dx*0.5)/d, -1.0, 1.0)
	return d * (intSmearFunc(zh, coeffs) - intSmearFunc(zl, coeffs)) / dx
}

func colorGb(img image.Image, srcScale, scale int, lcdMode int, prof DisplayProfile) image.Image {
	bounds := img.Bounds()
	srcWidth, srcHeight := bounds.Max.X, bounds.Max.Y

	// Need to accomodate for non integer scaling
	targetWidth, targetHeight := calculateScaledBufferSize(srcWidth, srcHeight, srcScale)
	// Linearize, downscale to real device resolution, and store
	buff := make([][]FloatColor, targetHeight)
	for y := 0; y < srcHeight; y += srcScale {
		row := make([]FloatColor, targetWidth)
		for x := 0; x < srcWidth; x += srcScale {
			row[x/srcScale] = rgbaToLinearColor(img.At(x, y).RGBA())
		}
		buff[y/srcScale] = row
	}

	srcWidth, srcHeight = targetWidth, targetHeight
	width, height := srcWidth*scale, srcHeight*scale

	srcWidthF := float64(srcWidth)
	srcHeightF := float64(srcHeight)

	// Image point sampler
	pointSampleBuff := func(x, y float64) FloatColor {
		x *= srcWidthF
		y *= srcHeightF
		return buff[min(max(int(math.Floor(y)), 0), srcHeight-1)][min(max(int(math.Floor(x)), 0), srcWidth-1)]
	}
	// Image load sampler
	loadBuff := func(x, y int) FloatColor {
		return buff[min(max(y, 0), srcHeight-1)][min(max(x, 0), srcWidth-1)]
	}

	texelSizeX := 1.0 / srcWidthF
	texelSizeY := 1.0 / srcHeightF
	outTexelSizeX := 1.0 / float64(width)
	outTexelSizeY := 1.0 / float64(height)

	// LCDv2 consts
	// Integral of (1 - x^2 - x^4 + x^6)^2
	coeffs_x := [...]float64{1.0, -2.0 / 3.0, -1.0 / 5.0, 4.0 / 7.0, -1.0 / 9.0, -2.0 / 11.0, 1.0 / 13.0}
	// Integral of (1 - 2x^4 + x^6)^2
	coeffs_y := [...]float64{1.0, 0.0, -4.0 / 5.0, 2.0 / 7.0, 4.0 / 9.0, -4.0 / 11.0, 1.0 / 13.0}

	// Sameboy params
	colorLow := 0.8      // min 0, max 1.5
	colorHigh := 1.0     // min 0, max 1.5
	scanlineDepth := 0.1 // min 0, max 2.0

	out := image.NewNRGBA(image.Rect(0, 0, width, height))

	for y := 0; y < height; y += 1 {
		texCoordY := outTexelSizeY * (float64(y) + 0.5)
		for x := 0; x < width; x += 1 {
			texCoordX := outTexelSizeX * (float64(x) + 0.5)

			var p FloatColor

			if lcdMode == 1 {
				// LCD Grid v2 shader
				tliX := int(math.Floor(texCoordX*srcWidthF - 0.4999))
				tliY := int(math.Floor(texCoordY*srcHeightF - 0.4999))

				subpix := (texCoordX*srcWidthF - 0.4999 - float64(tliX)) * 3.0
				rsubpix := outTexelSizeX * srcWidthF * 3.0

				lcol := FloatColor{intSmear(subpix+1.0, rsubpix, 1.5, coeffs_x),
					intSmear(subpix, rsubpix, 1.5, coeffs_x),
					intSmear(subpix-1.0, rsubpix, 1.5, coeffs_x)}
				rcol := FloatColor{intSmear(subpix-2.0, rsubpix, 1.5, coeffs_x),
					intSmear(subpix-3.0, rsubpix, 1.5, coeffs_x),
					intSmear(subpix-4.0, rsubpix, 1.5, coeffs_x)}

				if prof.BGR {
					lcol.R, lcol.G, lcol.B = lcol.B, lcol.G, lcol.R
					rcol.R, rcol.G, rcol.B = rcol.B, rcol.G, rcol.R
				}

				subpix = texCoordY*srcHeightF - 0.4999 - float64(tliY)
				rsubpix = outTexelSizeY * srcHeightF
				tcol := intSmear(subpix, rsubpix, 0.63, coeffs_y)
				bcol := intSmear(subpix-1.0, rsubpix, 0.63, coeffs_y)

				topLeftColor := loadBuff(tliX, tliY).Mult(lcol).MultF(tcol)
				bottomRightColor := loadBuff(tliX+1, tliY+1).Mult(rcol).MultF(bcol)
				bottomLeftColor := loadBuff(tliX, tliY+1).Mult(lcol).MultF(bcol)
				topRightColor := loadBuff(tliX+1, tliY).Mult(rcol).MultF(tcol)

				p = topLeftColor.Add(bottomRightColor).Add(bottomLeftColor).Add(topRightColor)
			} else if lcdMode == 0 {
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
				_, posX := math.Modf(texCoordX * srcWidthF)
				_, posY := math.Modf(texCoordY * srcHeightF)
				_, subPosX := math.Modf(texCoordX * srcWidthF)
				_, subPosY := math.Modf(texCoordY * srcHeightF)

				center := pointSampleBuff(texCoordX, texCoordY)
				left := pointSampleBuff(texCoordX-texelSizeX, texCoordY)
				right := pointSampleBuff(texCoordX+texelSizeX, texCoordY)

				if posY < 1.0/6.0 {
					center = lerpColor(center, pointSampleBuff(texCoordX, texCoordY-texelSizeY), 0.5-subPosY*0.5)
					left = lerpColor(left, pointSampleBuff(texCoordX-texelSizeX, texCoordY-texelSizeY), 0.5-subPosY*0.5)
					right = lerpColor(right, pointSampleBuff(texCoordX+texelSizeX, texCoordY-texelSizeY), 0.5-subPosY*0.5)
					center = center.MultF(subPosY*scanlineDepth + (1.0 - scanlineDepth))
					left = left.MultF(subPosY*scanlineDepth + (1.0 - scanlineDepth))
					right = right.MultF(subPosY*scanlineDepth + (1.0 - scanlineDepth))
				} else if posY > 5.0/6.0 {
					center = lerpColor(center, pointSampleBuff(texCoordX, texCoordY+texelSizeY), subPosY*0.5)
					left = lerpColor(left, pointSampleBuff(texCoordX-texelSizeX, texCoordY+texelSizeY), subPosY*0.5)
					right = lerpColor(right, pointSampleBuff(texCoordX+texelSizeX, texCoordY+texelSizeY), subPosY*0.5)
					center = center.MultF((1.0-subPosY)*scanlineDepth + (1.0 - scanlineDepth))
					left = left.MultF((1.0-subPosY)*scanlineDepth + (1.0 - scanlineDepth))
					right = right.MultF((1.0-subPosY)*scanlineDepth + (1.0 - scanlineDepth))
				}

				midLeft := lerpColor(left, center, 0.5)
				midRight := lerpColor(right, center, 0.5)

				if posX < 1.0/6.0 {
					p = lerpColor(
						FloatColor{colorHigh * center.R, colorLow * center.G, colorHigh * left.B},
						FloatColor{colorHigh * center.R, colorLow * center.G, colorLow * left.B},
						subPosX)
				} else if posX < 2.0/6.0 {
					p = lerpColor(
						FloatColor{colorHigh * center.R, colorLow * center.G, colorLow * left.B},
						FloatColor{colorHigh * center.R, colorHigh * center.G, colorLow * midLeft.B},
						subPosX)
				} else if posX < 3.0/6.0 {
					p = lerpColor(
						FloatColor{colorHigh * center.R, colorHigh * center.G, colorLow * midLeft.B},
						FloatColor{colorLow * midRight.R, colorHigh * center.G, colorLow * center.B},
						subPosX)
				} else if posX < 4.0/6.0 {
					p = lerpColor(
						FloatColor{colorLow * midRight.R, colorHigh * center.G, colorLow * center.B},
						FloatColor{colorLow * right.R, colorHigh * center.G, colorHigh * center.B},
						subPosX)
				} else if posX < 5.0/6.0 {
					p = lerpColor(
						FloatColor{colorLow * right.R, colorHigh * center.G, colorHigh * center.B},
						FloatColor{colorLow * right.R, colorLow * midRight.G, colorHigh * center.B},
						subPosX)
				} else {
					p = lerpColor(
						FloatColor{colorLow * right.R, colorLow * midRight.G, colorHigh * center.B},
						FloatColor{colorHigh * right.R, colorLow * right.G, colorHigh * center.B},
						subPosX)
				}
			} else {
				// No grid
				p = pointSampleBuff(texCoordX, texCoordY)
			}

			// LCD Grid v2 creates out of range results, so clamp them before color correction
			p = p.Clamp01()
			p = colorCorrect(p, prof)
			p = p.Gamma()
			out.Set(x, y, p.NRGBA())
		}
	}

	return out
}
