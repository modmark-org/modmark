// FIXME: Replace with a normal ES module import once Firefox adds support for js modules in web workers
importScripts("./pkg/web_bindings.js");

let loaded = false;
wasm_bindgen("./pkg/web_bindings_bg.wasm").then(() => {
    loaded = true;
    postMessage({ type: "init" });
});

onmessage = (event) => {
    if (!loaded) return;

    try {
        let result;
        switch (event.data.type) {
            case "ast":
                result = wasm_bindgen.ast(event.data.source);
                break;
            case "ast_debug":
                result = wasm_bindgen.ast_debug(event.data.source);
                break;
            case "json_output":
                result = wasm_bindgen.json_output(event.data.source);
                break;
            case "package_info":
                result = wasm_bindgen.package_info(event.data.source);
                break;
            case "transpile":
                result = wasm_bindgen.transpile(event.data.source, event.data.format);
                break;
        }
        postMessage({ result: result, success: true, ...event.data });
    } catch (error) {
        console.log(error);
        postMessage({ error: error, success: false, ...event.data })
    }
};
