// noinspection JSFileReferences
import init, {ast, ast_debug, json_output, package_info, transpile} from "./pkg/web_bindings.js";

let loaded = false;
init().then(() => {
    loaded = true;
    postMessage({type: "init"});
})

onmessage = (event) => {
    if (!loaded) return;

    try {
        let result;
        switch (event.data.type) {
            case "ast":
                result = ast(event.data.source);
                break;
            case "ast_debug":
                result = ast_debug(event.data.source);
                break;
            case "json_output":
                result = json_output(event.data.source);
                break;
            case "package_info":
                result = package_info(event.data.source);
                break;
            case "transpile":
                result = transpile(event.data.source, event.data.format);
                break;
        }
        postMessage({ result: result, success: true, ...event.data });
    } catch (error) {
        console.log(error);
        postMessage({ error: error, success: false, ...event.data })
    }
};
