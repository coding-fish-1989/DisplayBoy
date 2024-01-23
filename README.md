# DisplayBoy
Web app to make GBC and GBA screenshots look closer to the real thing.

No support for GB at this moment, although I would like to at some stage.

Written in Go, compiled to WASM. All code runs locally on your browser, nothing is sent to a server.

## Building
```
GOOS=js GOARCH=wasm go build -o main.wasm
```

## Running Locally
```
goexec 'http.ListenAndServe(`:8080`, http.FileServer(http.Dir(`.`)))'
```

Open `http://localhost:8080/` on browser.
