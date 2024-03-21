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

## Building
```
wasm-pack build
```

## Running Locally
```
cd www
npm run start
```
