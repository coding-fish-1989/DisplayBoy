import * as wasm from "display-boy";

let fileInput = document.getElementById('fileInput');
let fileOutput = document.getElementById('fileOutput');
let gbFgColor = document.getElementById('gbCustomFg');
let gbFgOpacity = document.getElementById('gbCustomFgOpacity');
let gbBgColor = document.getElementById('gbCustomBg');
let scaling = document.getElementById('scaling');
let convertButton = document.getElementById('convertButton');
let errorText = document.getElementById('convError');

convertButton.onclick = function () {
    let files = fileInput.files;
    var fileReader = new FileReader();
    fileReader.onload = function () {
        let data = new Uint8Array(fileReader.result)

        var colorModeValue = parseInt(document.querySelector('input[name="colorMode"]:checked').value);

        if (colorModeValue < 3) {
            let resDataBase64 = wasm.processImageGb(colorModeValue, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        } else if (colorModeValue == 3) {
            let fgColor = gbFgColor.value;
            let bgColor = gbBgColor.value;
            let fgOpacity = gbFgOpacity.value;
            let resDataBase64 = wasm.processImageGbCustom(fgColor, fgOpacity, bgColor, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        } else if (colorModeValue <= 7) {
            let scalingVal = parseInt(scaling.value);
            let lcdModeVal = parseInt(document.querySelector('input[name="lcdMode"]:checked').value);
            let resDataBase64 = wasm.processImageGbc(scalingVal, lcdModeVal, colorModeValue - 4, data);
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
            let resDataBase64 = wasm.processImageCrt(scalingVal, explicitAspectRatio, parVal, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        }
    }
    fileReader.readAsArrayBuffer(files[0]);
};