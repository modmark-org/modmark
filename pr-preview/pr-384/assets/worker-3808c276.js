(function() {
  "use strict";
  importScripts("./../../web_bindings/web_bindings.js");
  importScripts("https://unpkg.com/comlink/dist/umd/comlink.js");
  const compiler = {
    loaded: false,
    async init() {
      await wasm_bindgen("./../../web_bindings/web_bindings_bg.wasm");
      this.loaded = true;
    },
    blank_context() {
      if (!this.loaded)
        return;
      return wasm_bindgen.blank_context();
    },
    configure_from_source(source) {
      if (!this.loaded)
        return null;
      return wasm_bindgen.configure_from_source(source);
    },
    ast(source) {
      if (!this.loaded)
        return null;
      return wasm_bindgen.ast(source);
    },
    ast_debug(source) {
      if (!this.loaded)
        return null;
      return wasm_bindgen.ast_debug(source);
    },
    json_output(source) {
      if (!this.loaded)
        return null;
      return wasm_bindgen.json_output(source);
    },
    package_info(source) {
      if (!this.loaded)
        return null;
      return JSON.parse(wasm_bindgen.package_info(source));
    },
    transpile(source, format) {
      if (!this.loaded)
        return null;
      const result = wasm_bindgen.transpile(source, format);
      return JSON.parse(result);
    },
    transpile_no_document(source, format) {
      if (!this.loaded)
        return null;
      const result = wasm_bindgen.transpile_no_document(source, format);
      return JSON.parse(result);
    },
    // Note most of the following functions to interact with the virtual file system return strings
    // that may contain a error message. We may one to show those to users too!
    // (Rembember to change the type in compilerTypes.ts and FsTree.tsx if doing so) 
    add_file(path, bytes) {
      if (!this.loaded)
        return;
      wasm_bindgen.add_file(path, bytes);
    },
    add_folder(path) {
      if (!this.loaded)
        return;
      wasm_bindgen.add_folder(path);
    },
    rename_entry(from, to) {
      if (!this.loaded)
        return;
      wasm_bindgen.rename_entry(from, to);
    },
    remove_file(path) {
      if (!this.loaded)
        return;
      wasm_bindgen.remove_file(path);
    },
    remove_folder(path) {
      if (!this.loaded)
        return;
      wasm_bindgen.remove_folder(path);
    },
    read_file(path) {
      if (!this.loaded)
        return null;
      return wasm_bindgen.read_file(path);
    },
    get_file_list(path) {
      if (!this.loaded)
        return;
      return JSON.parse(wasm_bindgen.get_file_list(path));
    }
  };
  Comlink.expose(compiler);
})();
