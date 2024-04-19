import * as wasm from "display-boy";

let fileInput = document.getElementById('fileInput');
let fileOutput = document.getElementById('fileOutput');
let dither = document.getElementById('dither');
let brightness = document.getElementById('brightness');
let contrast = document.getElementById('contrast');
let edgeEnhancementLevel = document.getElementById('edgeEnhancementLevel');
let invert = document.getElementById('invert');
let gbFgColor = document.getElementById('gbCustomFg');
let gbFgOpacity = document.getElementById('gbCustomFgOpacity');
let gbBgColor = document.getElementById('gbCustomBg');
let scaling = document.getElementById('scaling');
let imageHeightCap = document.getElementById('imageHeightCap');
let imageDownsampleMethod = document.getElementById('imageDownsampleMethod');
let convertButton = document.getElementById('convertButton');
let errorText = document.getElementById('convError');

let brightnessRange = 100;
let outputBrightnessRange = 2.5;
let contrastRange = 100;

function lerp(a, b, t) {
    return a + t * (b - a);
}

function inv_lerp(a, b, t) {
    return (t - a) / (b - a);
}

function remap(value, low1, high1, low2, high2) {
    return lerp(low2, high2, inv_lerp(low1, high1, value));
}

fileInput.onchange = function () {
    let files = fileInput.files;
    if (files.length == 0) {
        convertButton.disabled = true;
        document.getElementById('deviceName').innerText = "";
        return;
    }
    var img = new Image();
    img.src = URL.createObjectURL(files[0]);
    img.onload = function () {
        let width = img.width;
        let height = img.height;

        let deviceName = wasm.getSourceDeviceName(width, height);
        document.getElementById('deviceName').innerText = "Source Device: " + deviceName;
        let recognizedSize = deviceName != "Unknown";
        if (recognizedSize) {
            imageHeightCap.classList.add("invisible");
            imageDownsampleMethod.classList.add("invisible");
        } else {
            imageHeightCap.classList.remove("invisible");
            imageDownsampleMethod.classList.remove("invisible");
        }

        convertButton.disabled = false;
    }
}

convertButton.onclick = function () {
    let files = fileInput.files;
    var fileReader = new FileReader();
    fileReader.onload = function () {
        let data = new Uint8Array(fileReader.result)

        var colorModeValue = parseInt(document.querySelector('input[name="colorMode"]:checked').value);
        var imageHeightCapFormValue = document.querySelector('input[name="imageHeightCap"]:checked').value;
        var imageDownsampleMethodValue = document.querySelector('input[name="imageDownsampleMethod"]:checked').value;

        var ditherValue = false;
        var brightnessValue = 1.0;
        var contrastValue = 1.0;
        var edgeEnhancementLevelValue = 0;
        var invertValue = false;

        if (document.getElementById('gbCameraMode').checked) {
            ditherValue = dither.checked;

            brightnessValue = parseInt(brightness.value);
            if (brightnessValue < 0) {
                brightnessValue = remap(brightnessValue, -brightnessRange, 0, 1.0 / outputBrightnessRange, 1);
            } else {
                brightnessValue = remap(brightnessValue, 0, brightnessRange, 1, outputBrightnessRange);
            }

            contrastValue = parseInt(contrast.value);
            contrastValue = 1.0 + contrastValue / contrastRange;

            edgeEnhancementLevelValue = parseInt(edgeEnhancementLevel.value);
            edgeEnhancementLevelValue = [0, 0.5, 0.75, 1, 1.25, 2, 3, 4, 5][edgeEnhancementLevelValue];

            invertValue = invert.checked;
        }


        var imageHeightCapValue = -1;
        if (imageHeightCapFormValue != "auto") {
            imageHeightCapValue = parseInt(imageHeightCapFormValue);
        }

        var requestBilinear = imageDownsampleMethodValue == "bilinear";

        if (colorModeValue < 3) {
            let resDataBase64 = wasm.processImageGb(colorModeValue, ditherValue, brightnessValue, contrastValue, invertValue, edgeEnhancementLevelValue, imageHeightCapValue, requestBilinear, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        } else if (colorModeValue == 3) {
            let fgColor = gbFgColor.value;
            let bgColor = gbBgColor.value;
            let fgOpacity = gbFgOpacity.value;
            let resDataBase64 = wasm.processImageGbCustom(fgColor, fgOpacity, bgColor, ditherValue, brightnessValue, contrastValue, invertValue, edgeEnhancementLevelValue, imageHeightCapValue, requestBilinear, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        } else if (colorModeValue <= 7) {
            let scalingVal = parseInt(scaling.value);
            let lcdModeVal = parseInt(document.querySelector('input[name="lcdMode"]:checked').value);
            let resDataBase64 = wasm.processImageGbc(scalingVal, lcdModeVal, colorModeValue - 4, imageHeightCapValue, requestBilinear, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        } else {
            let scalingVal = parseInt(scaling.value);
            let parVal = document.querySelector('input[name="par"]:checked').value;
            let explicitAspectRatio = false;
            if (parVal == "auto") {
                parVal = 0;
            } else {
                let aspectRatio = parVal.split(":");
                let aspectRatioX = parseFloat(aspectRatio[0]);
                let aspectRatioY = parseFloat(aspectRatio[1]);
                parVal = aspectRatioX / aspectRatioY;
                explicitAspectRatio = true;
            }
            let resDataBase64 = wasm.processImageCrt(scalingVal, explicitAspectRatio, parVal, imageHeightCapValue, requestBilinear, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        }
    }
    fileReader.readAsArrayBuffer(files[0]);
};