# DisplayBoy
Web app to make GB, GBC and GBA screenshots look closer to the real thing.

Written in Go, compiled to WASM. All code runs locally on your browser, nothing is sent to a server.

[Live Demo](https://coding-fish-1989.github.io/displayboy/)

## Notes
It runs on the CPU, and Go on WASM cannot perform tasks in parallel, which results in a significant performance hit. However, it is still fast enough for the purpose of this app.

## Building
```
GOOS=js GOARCH=wasm go build -ldflags="-s -w" -trimpath -o main.wasm
```

## Running Locally
```
goexec 'http.ListenAndServe(`:8080`, http.FileServer(http.Dir(`.`)))'
```

Open `http://localhost:8080/` on browser.
