rm -r build
mkdir build
cp -r static/* build
wasm-pack build ./web_bindings --target web --out-dir ../build/pkg
