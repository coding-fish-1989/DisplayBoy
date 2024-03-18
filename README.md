# DisplayBoy
Web app to make GB, GBC and GBA screenshots look closer to the real thing.

Written in Rust, compiled to WASM. All code runs locally on your browser, nothing is sent to a server.

[Live Demo](https://coding-fish-1989.github.io/displayboy/)

## Notes
It runs on the CPU, and Go on WASM cannot perform tasks in parallel, which results in a significant performance hit. However, it is still fast enough for the purpose of this app.

## Building
```
wasm-pack build
```

## Running Locally
```
cd www
npm run start
```
