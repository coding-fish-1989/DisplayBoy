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

func loadAlphaChecked(buff [][]float32, x, y, width, height int) float32 {
	if x < 0 || x >= width || y < 0 || y >= height {
		return 0
	}
	return buff[y][x]
}

func allocateAlphaBuff(width, height int) [][]float32 {
	buff := make([][]float32, height)
	for y := 0; y < height; y++ {
		buff[y] = make([]float32, width)
	}
	return buff
}

func bilinearScaleAlpha(fromWidth, fromHeight, toWidth, toHeight int, fromBuff, toBuff [][]float32) {
	xRatio := float32(fromWidth) / float32(toWidth)
	yRatio := float32(fromHeight) / float32(toHeight)
	for y := 0; y < toHeight; y++ {
		for x := 0; x < toWidth; x++ {
			xf := float32(x) * xRatio
			yf := float32(y) * yRatio
			xi := int(xf)
			yi := int(yf)
			xf -= float32(xi)
			yf -= float32(yi)
			toBuff[y][x] = (1.0-xf)*(1.0-yf)*loadAlphaChecked(fromBuff, xi, yi, fromWidth, fromHeight) +
				xf*(1.0-yf)*loadAlphaChecked(fromBuff, xi+1, yi, fromWidth, fromHeight) +
				(1.0-xf)*yf*loadAlphaChecked(fromBuff, xi, yi+1, fromWidth, fromHeight) +
				xf*yf*loadAlphaChecked(fromBuff, xi+1, yi+1, fromWidth, fromHeight)
		}
	}
}

func quantizeGb(l float64) float32 {
	borderError := 0.03 // To be lenient for some 4 color palette value
	// Custom intensity for each shade range
	if l <= 0.25+borderError {
		return 1.0
	} else if l <= 0.5+borderError {
		return 0.66666667
	} else if l <= 0.75+borderError {
		return 0.33333333
	}
	return 0.07
}

type GbDisplayProfile struct {
	ForegroundR, ForegroundG, ForegroundB, ForegroundA float64
	BackgroundR, BackgroundG, BackgroundB              float64
}

func gbMono(img image.Image, srcScale int, profile GbDisplayProfile) image.Image {
	bounds := img.Bounds()
	srcWidth, srcHeight := bounds.Max.X, bounds.Max.Y

	// Color configurations
	fg := FloatColor{profile.ForegroundR, profile.ForegroundG, profile.ForegroundB}.Linear()
	// Foreground color is not premultiplied on purpose since alpha is multiplied in foreground alpha quantization phase
	fgOpacity := profile.ForegroundA
	// Background color does not support opacity. This is because GB LCD has to be composited in linear space to look correct.
	// sRGB PNG is generated in this program, which means we can't really output correct image to be composited to abitary image..
	bg := FloatColor{profile.BackgroundR, profile.BackgroundG, profile.BackgroundB}.Linear()

	// Need to accomodate for non integer scaling
	targetWidth, targetHeight := calculateScaledBufferSize(srcWidth, srcHeight, srcScale)
	// Quantize to alpha, downscale to real device resolution, and store
	buff := make([][]float32, targetHeight)
	for y := 0; y < srcHeight; y += srcScale {
		row := make([]float32, targetWidth)
		for x := 0; x < srcWidth; x += srcScale {
			c := rgbaToLinearColor(img.At(x, y).RGBA())
			l := c.Luminance()
			// L* conversion
			if l <= (216.0 / 24389.0) {
				l = l * (24389.0 / 27.0)
			} else {
				l = math.Pow(l, (1.0/3.0))*116.0 - 16.0
			}
			// Normalize to [0,1]
			l /= 100.0
			alpha := quantizeGb(l)
			alpha *= float32(fgOpacity)
			row[x/srcScale] = alpha
		}
		buff[y/srcScale] = row
	}

	srcWidth, srcHeight = targetWidth, targetHeight

	// Don't change this without revising pretty much everything after this
	scale := 5

	width, height := srcWidth*scale, srcHeight*scale

	// Final output should have GB LCD margin
	// This margin also accounts for the edge smear and shadow blur expansion
	nativeMargin := 5
	margin := nativeMargin * scale
	outWidth, outHeight := width+margin*2, height+margin*2

	// Prepare buffers for separable gaussian blur
	// Should be about 2.46 MB on memory per buffer for full GB resolution with margin above
	bgPingBuff := allocateAlphaBuff(outWidth, outHeight)
	bgPongBuff := allocateAlphaBuff(outWidth, outHeight)

	// Upscale with grid line
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			// Gap between pixels. This code only works for resolution scale of 5.
			if (x%5) >= 4 || (y%5) >= 4 {
				continue
			}
			bgPingBuff[y+margin][x+margin] = buff[y/scale][x/scale]
		}
	}

	// Apply small blur to smear the pixel edges
	{
		// Subjective smear kernel
		var kernel = [...]float32{0.1, 0.8}

		// Horizontal gaussian blur pass
		for y := 0; y < outHeight; y++ {
			for x := 0; x < outWidth; x++ {
				g := float32(0)
				g += loadAlphaChecked(bgPingBuff, x-1, y, outWidth, outHeight) * kernel[0]
				g += loadAlphaChecked(bgPingBuff, x+0, y, outWidth, outHeight) * kernel[1]
				g += loadAlphaChecked(bgPingBuff, x+1, y, outWidth, outHeight) * kernel[0]
				bgPongBuff[y][x] = g
			}
		}

		bgPingBuff, bgPongBuff = bgPongBuff, bgPingBuff

		// Vertical gaussian blur pass
		for y := 0; y < outHeight; y++ {
			for x := 0; x < outWidth; x++ {
				g := float32(0)
				g += loadAlphaChecked(bgPingBuff, x, y-1, outWidth, outHeight) * kernel[0]
				g += loadAlphaChecked(bgPingBuff, x, y+0, outWidth, outHeight) * kernel[1]
				g += loadAlphaChecked(bgPingBuff, x, y+1, outWidth, outHeight) * kernel[0]
				bgPongBuff[y][x] = g
			}
		}

		bgPingBuff, bgPongBuff = bgPongBuff, bgPingBuff
	}

	// Gaussian blur result from above is now kept as a foreground buffer
	fgBuff := bgPingBuff

	// Allocate a new buffer for ping buffer, since it's been used for foreground buffer
	bgPingBuff = allocateAlphaBuff(outWidth, outHeight)

	// Apply larger blur for shadowing:
	// - Downsamples the foreground buffer to approximetely half the size
	// - Applies separable gaussian blur
	// - Upsamples the buffer back to original size
	// This is done to avoid using massive gaussian blur kernel.

	// Half the size, but round up. This is to avoid losing pixel data.
	outWidthSmall := (outWidth + 1) / 2
	outHeightSmall := (outHeight + 1) / 2

	// Downsample
	bilinearScaleAlpha(outWidth, outHeight, outWidthSmall, outHeightSmall, fgBuff, bgPingBuff)

	{
		// Gaussian kernel
		var kernel = [...]float32{0.006, 0.061, 0.241, 0.383}

		// Horizontal gaussian blur pass
		// It takes in blurred foreground buffer as an input to slightly increase the blur radius
		for y := 0; y < outHeightSmall; y++ {
			for x := 0; x < outWidthSmall; x++ {
				g := float32(0)
				g += loadAlphaChecked(bgPingBuff, x-3, y, outWidthSmall, outHeightSmall) * kernel[0]
				g += loadAlphaChecked(bgPingBuff, x-2, y, outWidthSmall, outHeightSmall) * kernel[1]
				g += loadAlphaChecked(bgPingBuff, x-1, y, outWidthSmall, outHeightSmall) * kernel[2]
				g += loadAlphaChecked(bgPingBuff, x+0, y, outWidthSmall, outHeightSmall) * kernel[3]
				g += loadAlphaChecked(bgPingBuff, x+1, y, outWidthSmall, outHeightSmall) * kernel[2]
				g += loadAlphaChecked(bgPingBuff, x+2, y, outWidthSmall, outHeightSmall) * kernel[1]
				g += loadAlphaChecked(bgPingBuff, x+3, y, outWidthSmall, outHeightSmall) * kernel[0]
				bgPongBuff[y][x] = g
			}
		}

		bgPingBuff, bgPongBuff = bgPongBuff, bgPingBuff

		// Vertical gaussian blur pass
		for y := 0; y < outHeightSmall; y++ {
			for x := 0; x < outWidthSmall; x++ {
				g := float32(0)
				g += loadAlphaChecked(bgPingBuff, x, y-3, outWidthSmall, outHeightSmall) * kernel[0]
				g += loadAlphaChecked(bgPingBuff, x, y-2, outWidthSmall, outHeightSmall) * kernel[1]
				g += loadAlphaChecked(bgPingBuff, x, y-1, outWidthSmall, outHeightSmall) * kernel[2]
				g += loadAlphaChecked(bgPingBuff, x, y+0, outWidthSmall, outHeightSmall) * kernel[3]
				g += loadAlphaChecked(bgPingBuff, x, y+1, outWidthSmall, outHeightSmall) * kernel[2]
				g += loadAlphaChecked(bgPingBuff, x, y+2, outWidthSmall, outHeightSmall) * kernel[1]
				g += loadAlphaChecked(bgPingBuff, x, y+3, outWidthSmall, outHeightSmall) * kernel[0]
				bgPongBuff[y][x] = g
			}
		}

		bgPingBuff, bgPongBuff = bgPongBuff, bgPingBuff
	}

	// Upsample
	bilinearScaleAlpha(outWidthSmall, outHeightSmall, outWidth, outHeight, bgPingBuff, bgPongBuff)

	bgShadowBuff := bgPongBuff

	// Invalidate temporary buffers
	bgPingBuff = nil
	bgPongBuff = nil

	shadowOpacity := 0.5
	shadowOffset := 1

	out := image.NewNRGBA(image.Rect(0, 0, outWidth, outHeight))
	for y := 0; y < outWidth; y++ {
		for x := 0; x < outWidth; x++ {
			// Background shadowing
			shadow := float64(loadAlphaChecked(bgShadowBuff, x-shadowOffset, y-shadowOffset, outWidth, outHeight))
			shadow *= shadowOpacity
			c := bg.MultF(1.0 - shadow)
			// Alpha blend foreground
			opacity := float64(loadAlphaChecked(fgBuff, x, y, outWidth, outHeight))
			c = fg.MultF(opacity).Add(c.MultF(1.0 - opacity))
			// Gamma compression
			c = c.Gamma()
			out.Set(x, y, c.NRGBA())
		}
	}
	return out
}
