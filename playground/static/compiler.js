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
            case "transpile_no_document":
                result = wasm_bindgen.transpile_no_document(event.data.source, event.data.format);
                break;
            case "add_file":
                wasm_bindgen.add_file(event.data.path, event.data.bytes);
                break;
            case "add_folder":
                wasm_bindgen.add_folder(event.data.path);
                break;
            case "rename_entry":
                wasm_bindgen.rename_entry(event.data.from, event.data.to);
                break;
            case "remove_file":
                wasm_bindgen.remove_file(event.data.path);
                break;
            case "remove_dir":
                wasm_bindgen.remove_dir(event.data.path);
                break;
            case "get_file_list":
                result = wasm_bindgen.get_file_list(event.data.path);
                break;
        }
        postMessage({ result: result, success: true, ...event.data });
    } catch (error) {
        console.log(error);
        postMessage({ error: error, success: false, ...event.data })
    }
};
