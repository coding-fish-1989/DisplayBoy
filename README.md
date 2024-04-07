# DisplayBoy
Web app to convert GB, GBC, and GBA screenshots to resemble the display of the actual device more closely.

Written in Rust, compiled to WASM. All code runs locally on your browser, nothing is sent to a server.

[Live Demo](https://coding-fish-1989.github.io/displayboy/)

## Avaibale Modes
- GB
- GBP
- GBL
- GB Custom (allows you to configure colors)
- GBC
- GBA
- GBA SP
- GBA SP White (a variant of GBA SP mode)
- CRT

## GB Camera Mode
This mode is used to apply similar effects to the GB Camera, such as dithering, brightness, contrast, and edge enhancements. It can be used to make modern photos resemble those shot using the GB Camera.

## Building
```
wasm-pack build
```

## Running Locally
```
cd www
npm run start
```
