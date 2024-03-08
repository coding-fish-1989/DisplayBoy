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
	"bytes"
	_ "embed"
	b64 "encoding/base64"
	"errors"
	"fmt"
	"image"
	_ "image/jpeg"
	"image/png"
	"strconv"
	"syscall/js"
)

// Optimize color linearization with LUT
var linearTable []float64

func PrepareLinearTable() {
	linearTable = make([]float64, 256)
	for i := 0; i < 256; i++ {
		linearTable[i] = linear(float64(i) / float64(0xff))
	}
}

func execute(input []byte, colorMode, lcdMode, scale int, gbBgColor, gbFgColor FloatColor, gbFgOpacity float64) ([]byte, error) {
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
		// GB aspect ratio
		mult = width / 160
		fmt.Println("GB")
	}

	if (width/mult*scale)*(height/mult*scale) > (240 * 160 * 8 * 8) {
		return nil, errors.New("The expected output image size is too large")
	}

	gb := GbDisplayProfile{
		19.0 / 255.0, 74.0 / 255.0, 7.0 / 255.0, 1.0,
		170.0 / 255.0, 181.0 / 255.0, 19.0 / 255.0,
	}

	gbp := GbDisplayProfile{
		0.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0, 1.0,
		164.0 / 255.0, 169.0 / 255.0, 137.0 / 255.0,
	}

	gbl := GbDisplayProfile{
		0.0 / 255.0, 46.0 / 255.0, 44.0 / 255.0, 1.0,
		0.0 / 255.0, 181.0 / 255.0, 176.0 / 255.0,
	}

	gbCustom := GbDisplayProfile{
		gbFgColor.R, gbFgColor.G, gbFgColor.B, gbFgOpacity,
		gbBgColor.R, gbBgColor.G, gbBgColor.B,
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

	var out image.Image

	if colorMode <= 3 {
		var prof GbDisplayProfile

		switch colorMode {
		case 0:
			prof = gb
		case 1:
			prof = gbp
		case 2:
			prof = gbl
		case 3:
			prof = gbCustom
		}

		out = gbMono(img, mult, prof)
	} else if colorMode <= 7 {
		colorMode -= 4

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

		prof = prof

		out = colorGb(img, mult, scale, lcdMode, prof)
	} else {
		out = crt(img, mult, 6)
	}

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
	gbFgColor := document.Call("getElementById", "gbCustomFg")
	gbFgOpacity := document.Call("getElementById", "gbCustomFgOpacity")
	gbBgColor := document.Call("getElementById", "gbCustomBg")
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

			gbFgColorHex, _ := strconv.ParseUint(gbFgColor.Get("value").String()[1:], 16, 32)
			gbFgColorValue := FloatColor{float64(gbFgColorHex>>16) / float64(0xff), float64((gbFgColorHex>>8)&0xff) / float64(0xff), float64(gbFgColorHex&0xff) / float64(0xff)}
			gbFgOpacityValue, _ := strconv.Atoi(gbFgOpacity.Get("value").String())

			gbBgColorHex, _ := strconv.ParseUint(gbBgColor.Get("value").String()[1:], 16, 32)
			gbBgColorValue := FloatColor{float64(gbBgColorHex>>16) / float64(0xff), float64((gbBgColorHex>>8)&0xff) / float64(0xff), float64(gbBgColorHex&0xff) / float64(0xff)}

			scaling, _ := strconv.Atoi(scaling.Get("value").String())
			scaling = min(max(scaling, 1), 8)

			dst, err := execute(dst, colorMode.Get("selectedIndex").Int(), lcdMode.Get("selectedIndex").Int(), scaling, gbBgColorValue, gbFgColorValue, float64(gbFgOpacityValue)/100.0)

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
