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
   - And obviously written in Go
*/

package main

import (
	"image"
	"math"
)

const crtMargin = 4
const crtScanlineWeight = 0.3
const crtLum = 0.0
const crtDotMask = 0.2
const crtGamma = 2.4
const crtLanczosSize = 2

const crtPwr = 1.0/((-0.7*(1.0-crtScanlineWeight)+1.0)*(-0.5*crtDotMask+1.0)) - 1.25

func scanlineWeights(distance float64, color FloatColor) FloatColor {
	wid := color.Pow4().MultF(2.0).AddF(2.0)
	weights := makeFloatColor(distance / crtScanlineWeight)
	return (wid.MultF(0.5).PowF(-0.5).Mult(weights).Pow(wid).MultF(-1).Exp()).MultF(crtLum + 1.4).Div(wid.MultF(0.2).AddF(0.6))
}

// Returns gamma corrected output, compensated for scanline+mask embedded gamma
func crtInvGamma(col FloatColor) FloatColor {
	cir := col.SubF(1.0)
	cir = cir.Mult(cir)
	return lerpColor(col.Sqrt(), (makeFloatColor(1.0).Sub(cir)).Sqrt(), crtPwr)
}

func crt(img image.Image, srcScale, scale int) image.Image {
	bounds := img.Bounds()
	srcWidth, srcHeight := bounds.Max.X, bounds.Max.Y

	topMargin := 0
	// SNES has 224 height, but CRT is 240. Insert margins.
	if srcHeight < 240 && srcHeight >= 224 {
		topMargin = (240 - srcHeight) / 2
		srcHeight = 240
	}

	// Add frame margin
	scaledMargin := crtMargin * srcScale
	topMargin += scaledMargin
	leftMargin := scaledMargin
	srcHeight += scaledMargin * 2
	srcWidth += scaledMargin * 2

	// Need to accomodate for non integer scaling
	targetWidth, targetHeight := calculateScaledBufferSize(srcWidth, srcHeight, srcScale)

	// Linearize, downscale to real device resolution, and store
	buff := make([][]FloatColor, targetHeight)
	for y := 0; y < srcHeight; y += srcScale {
		row := make([]FloatColor, targetWidth)
		for x := 0; x < srcWidth; x += srcScale {
			row[x/srcScale] = rgbaToLinearColor(img.At(x-leftMargin, y-topMargin).RGBA())
		}
		buff[y/srcScale] = row
	}

	srcWidth, srcHeight = targetWidth, targetHeight
	width, height := srcWidth*scale, srcHeight*scale

	srcWidthF := float64(srcWidth)
	srcHeightF := float64(srcHeight)

	// Image load sampler
	loadBuff := func(x, y int) FloatColor {
		if y < 0 || y >= srcHeight || x < 0 || x >= srcWidth {
			return makeFloatColor(0.0)
		}
		return buff[y][x]
	}

	outTexelSizeX := 1.0 / float64(width)
	outTexelSizeY := 1.0 / float64(height)

	srcTexelSizeX := 1.0 / float64(srcWidth)

	// float filter_ = InputSize.y/OutputSize.y;
	filter := 1.0 / float64(scale)

	out := image.NewNRGBA(image.Rect(0, 0, width, height))

	for y := 0; y < height; y += 1 {
		// This is supposed to be correct, but have been offset by 0.5 for output to better
		// align the line between the scanlines.
		// texCoordY := outTexelSizeY * (float64(y) + 0.5)
		texCoordY := outTexelSizeY * (float64(y) + 0.0)

		ratioScaleY := texCoordY*srcHeightF - 0.5
		yy := math.Floor(ratioScaleY)
		uvRatioYOrig := ratioScaleY - yy

		for x := 0; x < width; x += 1 {
			texCoordX := outTexelSizeX * (float64(x) + 0.5)

			ratioScaleX := texCoordX*srcWidthF - 0.5
			xx := math.Floor(ratioScaleX)
			uvRatioX := ratioScaleX - xx

			col := makeFloatColor(0.0)
			col2 := makeFloatColor(0.0)

			// Horizontal only Lanczos
			{
				// Doesn't include half texel offset for Lanczos
				outTexCoordX := float64(x) * outTexelSizeX
				srcTexCoordX := float64(xx) * srcTexelSizeX
				for lx := -crtLanczosSize; lx <= crtLanczosSize; lx++ {
					srcSampleTexCoordX := srcTexCoordX + float64(lx)*srcTexelSizeX
					d := clamp((srcSampleTexCoordX-outTexCoordX)*srcWidthF, float64(-crtLanczosSize), float64(crtLanczosSize))

					w := 1.0
					if d != 0.0 {
						w = (float64(crtLanczosSize) * math.Sin(math.Pi*d) * math.Sin(math.Pi*(d/float64(crtLanczosSize)))) / (math.Pi * math.Pi * d * d)
					}

					val := loadBuff(int(xx)+lx, int(yy))
					col = col.Add(val.MultF(w))

					val = loadBuff(int(xx)+lx, int(yy)+1)
					col2 = col2.Add(val.MultF(w))
				}
			}

			col = col.Clamp01()
			col2 = col2.Clamp01()

			uvRatioY := uvRatioYOrig

			weights := scanlineWeights(uvRatioY, col)
			weights2 := scanlineWeights(1.0-uvRatioY, col2)
			uvRatioY = uvRatioY + 1.0/3.0*filter

			weights = weights.Add(scanlineWeights(uvRatioY, col)).DivF(3.0)
			weights2 = weights2.Add(scanlineWeights(math.Abs(1.0-uvRatioY), col2)).DivF(3.0)
			uvRatioY = uvRatioY - 2.0/3.0*filter

			weights = weights.Add(scanlineWeights(math.Abs(uvRatioY), col).DivF(3.0))
			weights2 = weights2.Add(scanlineWeights(math.Abs(1.0-uvRatioY), col2).DivF(3.0))

			mulRes := col.Mult(weights).Add(col2.Mult(weights2))

			// Dotmask
			maskGreenWeight := 1.0 - math.Abs(uvRatioX*2.0-1.0)
			dotMaskWeights := lerpColor(makeFloatColor3(1.0, 1.0-crtDotMask, 1.0), makeFloatColor3(1.0-crtDotMask, 1.0, 1.0-crtDotMask), maskGreenWeight)
			mulRes = mulRes.Mult(dotMaskWeights).Clamp01()

			mulRes = crtInvGamma(mulRes)

			p := mulRes.Clamp01()

			out.Set(x, y, p.NRGBA())
		}
	}

	return out
}
