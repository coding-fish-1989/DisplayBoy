import * as wasm from "display-boy";

let fileInput = document.getElementById('fileInput');
let fileOutput = document.getElementById('fileOutput');
let colorMode = document.getElementById('colorMode');
let gbFgColor = document.getElementById('gbCustomFg');
let gbFgOpacity = document.getElementById('gbCustomFgOpacity');
let gbBgColor = document.getElementById('gbCustomBg');
let lcdMode = document.getElementById('lcdMode');
let scaling = document.getElementById('scaling');
let convertButton = document.getElementById('convertButton');
let errorText = document.getElementById('convError');

convertButton.onclick = function () {
    let files = document.getElementById('fileInput').files;
    var fileReader = new FileReader();
    fileReader.onload = function () {
        let data = new Uint8Array(fileReader.result)

        let colorModeValue = colorMode.selectedIndex;
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
            let scalingVal = scaling.value;
            let lcdModeVal = lcdMode.selectedIndex;
            let resDataBase64 = wasm.processImageGbc(scalingVal, lcdModeVal, colorModeValue - 4, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        } else {
            let resDataBase64 = wasm.processImageCrt(6, data);
            fileOutput.src = "data:image/png;base64," + resDataBase64;
        }
    }
    fileReader.readAsArrayBuffer(files[0]);
};