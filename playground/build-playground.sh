rm -r build
mkdir build
cp -r static/* build
wasm-pack build ./web_bindings --target no-modules --out-dir ../build/pkg
