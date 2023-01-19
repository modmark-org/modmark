To build the playground run
```sh
wasm-pack build ./web_bindings --target web --out-dir ../pkg
```

Then you can simply host `index.html` on a static file server (sadly you can't load wasm when opening a local html file). A simple way to host the file is by using live-server.
```sh
npm install -g live-server

live-server
```
