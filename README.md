# DisplayBoy
Web app to make GBC and GBA screenshots look closer to the real thing.

No support for GB at this moment, although I would like to at some stage.

Written in Go, compiled to WASM. All code runs locally on your browser, nothing is sent to a server.

[Live Demo](https://coding-fish-1989.github.io/displayboy/)

## Notes
Most of the image operations are ported from various shaders. They all run on the CPU instead of the GPU, and Go on WASM cannot perform tasks in parallel, which results in a significant performance hit. However, it is still fast enough for the purpose of this app.

## Building
```
GOOS=js GOARCH=wasm go build -o main.wasm
```

## Running Locally
```
goexec 'http.ListenAndServe(`:8080`, http.FileServer(http.Dir(`.`)))'
```

Open `http://localhost:8080/` on browser.
