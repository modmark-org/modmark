The instructions below will build and host the static site in the directory `build`:

1. First, install [wasm-pack](https://rustwasm.github.io/wasm-pack/) (and
   optionally [live-server](https://www.npmjs.com/package/live-server) for local hosting):
    ```shell
    cargo install wasm-pack
    npm install -g live-server
    ```

2. Then, make the build directory:
    ```shell
    mkdir build
    ```

3. Copy the static files and create the wasm bindings:
    ```sh
    cp -r static/* build
    wasm-pack build ./web_bindings --target web --out-dir ../build/pkg
    ```
   (Or just run the `build-playground.sh` script)

4. Then you can simply host the static files on a file server (sadly you can't load wasm when opening a local html
   file). A simple way to host the file is by using live-server.
    ```sh
    live-server build
    ```

There is an action workflow running on all pushes to `main` which will build and deploy the playground to [modmark-org.github.io/modmark](https://modmark-org.github.io/modmark/).
