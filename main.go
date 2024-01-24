package main

import (
	"bytes"
	_ "embed"
	b64 "encoding/base64"
	"errors"
	"fmt"
	"image"
	"image/color"
	_ "image/jpeg"
	"image/png"
	"math"
	"strconv"
	"syscall/js"
)

type DisplayProfile struct {
	R, GR, BR, RG, G, BG, RB, GB, B, Lum float64
	Gamma, GammaOffset                   float64
	BGR                                  bool
}

func colorCorrect(c FloatColor, p DisplayProfile) FloatColor {
	// Compress to profile gamma, offset and bring it back to linear
	c = c.Pow((p.Gamma + p.GammaOffset) / p.Gamma)
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

func lcdGrid(img image.Image, srcScale, scale int, lcdMode int, prof DisplayProfile) image.Image {
	bounds := img.Bounds()
	srcWidth, srcHeight := bounds.Max.X, bounds.Max.Y

	// Linearize, downscale to real device resolution, and store
	buff := make([][]FloatColor, srcHeight/srcScale)
	for y := 0; y < srcHeight; y += srcScale {
		row := make([]FloatColor, srcWidth/srcScale)
		for x := 0; x < srcWidth; x += srcScale {
			row[x/srcScale] = rgbaToLinearColor(img.At(x, y).RGBA())
		}
		buff[y/srcScale] = row
	}

	srcWidth /= srcScale
	srcHeight /= srcScale
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

func (p FloatColor) Add(c FloatColor) FloatColor {
	return FloatColor{p.R + c.R, p.G + c.G, p.B + c.B}
}

func (p FloatColor) AddF(f float64) FloatColor {
	return FloatColor{p.R + f, p.G + f, p.B + f}
}

func (p FloatColor) Sub(rhs FloatColor) FloatColor {
	return FloatColor{p.R - rhs.R, p.G - rhs.G, p.B - rhs.B}
}

func clamp(f, low, high float64) float64 {
	return min(max(f, low), high)
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

func (p FloatColor) Pow(e float64) FloatColor {
	return FloatColor{math.Pow(p.R, e), math.Pow(p.G, e), math.Pow(p.B, e)}
}

func linear(v float64) float64 {
	if v <= 0.04045 {
		return v * (1.0 / 12.92)
	}
	return math.Pow((v+0.055)/1.055, 2.4)
}

func (p FloatColor) Linear() FloatColor {
	return FloatColor{linear(p.R), linear(p.G), linear(p.B)}
}

func gamma(v float64) float64 {
	if v <= 0.0031308 {
		return v * 12.92
	}
	return 1.055*math.Pow(v, 1.0/2.4) - 0.055
}

func (p FloatColor) Gamma() FloatColor {
	return FloatColor{gamma(p.R), gamma(p.G), gamma(p.B)}
}

func lerpColor(l, r FloatColor, t float64) FloatColor {
	return l.Add(r.Sub(l).MultF(t))
}

func floatToByte(v float64) uint8 {
	if v >= 1 {
		return 255
	}
	return uint8(math.Floor(v * 256))
}

func (p FloatColor) NRGBA() color.NRGBA {
	return color.NRGBA{
		R: floatToByte(p.R),
		G: floatToByte(p.G),
		B: floatToByte(p.B),
		A: 255,
	}
}

// Optimize color linearization with LUT
var linearTable []float64

func PrepareLinearTable() {
	linearTable = make([]float64, 256)
	for i := 0; i < 256; i++ {
		linearTable[i] = linear(float64(i) / float64(0xff))
	}
}

func rgbaToLinearColor(r, g, b, a uint32) FloatColor {
	return FloatColor{linearTable[r>>8], linearTable[g>>8], linearTable[b>>8]}
}

func approximetlyEqual(a, b float64) bool {
	tolerance := 0.001
	diff := math.Abs(a - b)
	return diff < tolerance
}

func execute(input []byte, colorMode, lcdMode, scale int) ([]byte, error) {
	img, _, err := image.Decode(bytes.NewReader(input))
	if err != nil {
		return nil, err
	}

	bounds := img.Bounds()
	width, height := bounds.Max.X, bounds.Max.Y
	mult := 1

	// Detect device type from aspect ratio
	// The derived screen dimension is used to find out source scaling multiplier
	// This is needed because some apps integer scale screenshots on export
	if approximetlyEqual(float64(width)/float64(height), 1.5) {
		// GBA aspect ratio
		mult = width / 240
		fmt.Println("GBA")
	}
	if approximetlyEqual(float64(width)/float64(height), 1.11111111111111111111) {
		// GBC aspect ratio
		mult = width / 160
		fmt.Println("GBC")
	}

	if (width/mult*scale)*(height/mult*scale) > (240 * 160 * 8 * 8) {
		return nil, errors.New("The expected output image size is too large")
	}

	// Pokefan531's display profiles
	// https://forums.libretro.com/t/real-gba-and-ds-phat-colors/1540/220
	gbc := DisplayProfile{
		0.80, 0.275, -0.075, 0.135, 0.64, 0.225, 0.195, 0.155, 0.65, 0.93,
		2.2, -0.5,
		false,
	}
	gba := DisplayProfile{
		0.80, 0.275, -0.075, 0.135, 0.64, 0.225, 0.195, 0.155, 0.65, 0.93,
		2.0, 0.5,
		true,
	}
	gbaSp := DisplayProfile{
		0.86, 0.10, -0.06, 0.03, 0.745, 0.0675, 0.0025, -0.03, 1.0275, 0.97,
		2.0, 0.0,
		false,
	}
	gbaSpWhite := DisplayProfile{
		0.955, 0.11, -0.065, 0.0375, 0.885, 0.0775, 0.0025, -0.03, 1.0275, 0.94,
		2.0, 0.0,
		false,
	}

	var prof DisplayProfile

	switch colorMode {
	case 0:
		prof = gbc
	case 1:
		prof = gba
	case 2:
		prof = gbaSp
	case 3:
		prof = gbaSpWhite
	}

	out := lcdGrid(img, mult, scale, lcdMode, prof)

	buf := new(bytes.Buffer)
	err = png.Encode(buf, out)
	return buf.Bytes(), err
}

func main() {
	quit := make(chan struct{}, 0)

	PrepareLinearTable()

	document := js.Global().Get("document")

	fileInput := document.Call("getElementById", "fileInput")
	fileOutput := document.Call("getElementById", "fileOutput")
	colorMode := document.Call("getElementById", "colorMode")
	lcdMode := document.Call("getElementById", "lcdMode")
	scaling := document.Call("getElementById", "scaling")
	convertButton := document.Call("getElementById", "convertButton")
	errorText := document.Call("getElementById", "convError")

	convertButton.Set("onclick", js.FuncOf(func(v js.Value, x []js.Value) any {
		file := fileInput.Get("files").Call("item", 0)
		if file.IsNull() {
			return nil
		}
		file.Call("arrayBuffer").Call("then", js.FuncOf(func(v js.Value, x []js.Value) any {
			data := js.Global().Get("Uint8Array").New(x[0])
			dst := make([]byte, data.Get("length").Int())
			js.CopyBytesToGo(dst, data)

			scaling, _ := strconv.Atoi(scaling.Get("value").String())
			scaling = min(max(scaling, 1), 8)
			dst, err := execute(dst, colorMode.Get("selectedIndex").Int(), lcdMode.Get("selectedIndex").Int(), scaling)

			if err == nil {
				sEnc := b64.StdEncoding.EncodeToString([]byte(dst))
				fileOutput.Set("src", "data:image/png;base64,"+sEnc)
				errorText.Set("innerHTML", "")
			} else {
				// Blank gif
				fileOutput.Set("src", "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7")
				errorText.Set("innerHTML", err.Error())
			}

			return nil
		}))

		return nil
	}))

	<-quit
}
