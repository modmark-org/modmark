let seq = 0;
let compiler_callback;
let compiler_failure;
let compiler = new Worker("./compiler.js");


compiler.onmessage = (event) => {
    // Render the document once the wasm module containing the
    // compiler has been instantiated.
    if (event.data.type === "init") {
        updateOutput(editor.getValue());
        return;
    }

    if (event.data.seq !== seq) return;
    if (event.data.success) {
        compiler_callback(event.data.result);
    } else {
        compiler_failure(event.data.error);
    }
}

async function compilerAction(action) {
    let promise = new Promise((res, rej) => {
        compiler_callback = res;
        compiler_failure = rej;
    });
    compiler.postMessage({ seq: ++seq, ...action });
    return promise;
}

let view = "editor";
let fileMenuVisible = false;
let currentPath = "/";
let selectedEntry = "";
let folderCount = 0;

const editorView = document.getElementById("editor-view");

// Set up the editor
const editorOptions = {
    fontFamily: "IBM Plex Mono",
    fontSize: "12pt"
};

let editor = ace.edit("editor");
editor.setOptions(editorOptions);
editor.session.setUseWrapMode(true);

// Load the example document async, not to freeze loading the rest of the playground
fetch("example.mdm").then(res => res.text().then(text => {
    editor.session.setValue(text);
    updateOutput(text);
}));

// Warnings and errors
const errorLog = document.getElementById("error-log");
const errorPrompt = document.getElementById("error-prompt");
const warningPrompt = document.getElementById("warning-prompt");
const warningLog = document.getElementById("warning-log");

// Output editor
const debugEditor = ace.edit("debug-editor");
debugEditor.setOptions(editorOptions);
debugEditor.setReadOnly(true);
debugEditor.setHighlightActiveLine(false);
debugEditor.container.style.background = "#f5f5f5"

// Output rendering
const render = document.getElementById("render");
const renderIframe = document.getElementById("render-iframe");
const status = document.getElementById("status");

// Package view
const packageView = document.getElementById("package-view");
const packageContent = document.getElementById("package-content");

// Files
const fileMenu = document.getElementById("file-menu");
const fileList = document.getElementById("file-list");
const fileUpload = document.getElementById("file-upload")
const folderButton = document.getElementById("folder-button");
const returnButton = document.getElementById("return-button")
const currentFolder = document.getElementById("current-folder");

// Menu options
const selector = document.getElementById("selector");
const viewToggle = document.getElementById("view-toggle");
const leftMenu = document.getElementById("left-menu");
const formatInput = document.getElementById("format-input");
const viewFiles = document.getElementById("view-files");

// Set to "render html" by default
selector.value = "render";

if (selector.value === "transpile-other") {
    formatInput.style.display = "block";
} else {
    formatInput.style.display = "none";
}

viewToggle.onclick = toggleView;
viewFiles.onclick = toggleFileMenu;
fileUpload.onchange = handleFileUpload;
folderButton.onclick = addFolder;
returnButton.onclick = leaveDir;

// Add the PR button
const regex = /pr-preview\/pr-(\d+)/;
const match = document.location.href.match(regex);
if (match !== null) {
    const branch = match[1]
    const button = document.createElement("button");
    button.innerHTML = buttonContent("Previewing #" + branch, "alt_route");
    button.onclick = () => window.location = `https://github.com/modmark-org/modmark/pull/${branch}`;
    leftMenu.appendChild(button);
}

editor.session.on("change", (_event) => handleChange());
selector.onchange = handleMenuUpdate;
formatInput.onchange = handleMenuUpdate;

function handleMenuUpdate(_event) {
    if (selector.value === "transpile-other") {
        formatInput.style.display = "block";
    } else {
        formatInput.style.display = "none";
    }
    updateOutput(editor.getValue());
}

let timeoutId = null;

function handleChange() {
    if (timeoutId !== null) {
        clearTimeout(timeoutId);
    }

    status.innerHTML = buttonContent("Typing…", "keyboard");

    timeoutId = setTimeout(() => {
        updateOutput(editor.getSession().getValue());
    }, 500);
}


function buttonContent(text, icon) {
    return `
            <span class="material-symbols-outlined">
                ${icon}
            </span>
            ${text}
            `;
}


function toggleView() {
    switch (view) {
        case "editor":
            // open the package view
            view = "package";
            packageView.style.width = "100%";
            editorView.style.width = "0";
            viewToggle.innerHTML = buttonContent("View editor", "edit");
            selector.setAttribute("disabled", "true");
            fileMenuVisible = true;
            toggleFileMenu();
            loadPackageInfo();
            break;
        case "package":
            // close the package view and return to the editor
            view = "editor";
            packageView.style.width = "0";
            editorView.style.width = "100%";
            viewToggle.innerHTML = buttonContent("View packages", "widgets");
            selector.removeAttribute("disabled");
            break;
    }
}

async function loadPackageInfo() {
    const info = JSON.parse(await compilerAction({ type: "package_info" }));

    const type_annotation = (type) => {
        let type_name = type;
        let color = "";
        if (Array.isArray(type)) {
            type_name = type.join("/");
            color = "#90A959";
        } else if (type == "String") {
            color = "#2A6041";
        } else if (type == "Unsigned integer") {
            color = "#3581B8";
        } else if (type == "Integer") {
            color = "#8B85C1";
        } else if (type == "Float") {
            color = "#C3423F";
        }

        return `<span style="background-color: ${color};" class="type">${type_name}</span>`
    }

    const escape_default = (type, default_value) => {
        if (Array.isArray(type) || type == "String") {
            return `"${default_value}"`;
        } else {
            return default_value;
        }
    }

    const createTransformList = (transform) => {
        let args = transform.arguments.map((arg) => `<div class="argument">
            <div class="name-container"><span class="name">${arg.name}</span>${type_annotation(arg.type)}</div>
            <div class="default">${arg.default !== null ? 'default = ' + escape_default(arg.type, arg.default) : 'required'}</div>
            <div class="description">${arg.description}</div>
        </div>`).join("\n");

        return `<div class="transform">
            <div class="transform-heading">
                <code class="from">${transform.from}</code>
                <div class="to">
                    supports
                    ${transform.to.length > 0 ? transform.to.map(t => `<span>${t}</span>`).join("") : "all"}
                </div>
            </div>
            <div class="transform-description">${transform.description ?? ""}</div>
            <div class="arguments">${args}</div>
        </div> `
    };

    const createElem = ({ name, version, description, transforms }) => {
        let expanded = false;

        let container = document.createElement("div");
        container.classList.add("container");

        container.innerHTML = `
                <h3 class="name">${name}</h3>
                <span class="version">version ${version}</span>
                <p class="description">
                    ${description}
                </p> 
                <div class="details">
                    <h4>Transforms</h4>
                    <div class="transforms">
                        ${transforms.sort((a, b) => a.name < b.name).map(createTransformList).join("\n")}
                    </div>
                </div>
            </div>
        `;

        container.onclick = (_event) => {
            if (expanded) {
                container.classList.remove("expanded");
            } else {
                container.classList.add("expanded");
            }
            expanded = !expanded;
        }

        return container;
    }

    packageContent.innerHTML = "";
    info.map(info => createElem(info)).forEach(elem => packageContent.appendChild(elem));

}

function toggleFileMenu() {
    if (fileMenuVisible) {
        fileMenu.style.width = "0";
    } else {
        fileMenu.style.width = "24rem";
    }
    fileMenuVisible = !fileMenuVisible;
}

async function handleFileUpload() {
    const uploads = fileUpload.files;
    for (let i = 0; i < uploads.length; i++) {
        const promise = uploads[i].arrayBuffer();
        promise.then(
            async function (result) {
                const bytes = new Uint8Array(result);
                await compilerAction({
                    type: "add_file",
                    path: currentPath + uploads[i].name,
                    bytes: bytes
                })
                await updateFileList();
            }
        ).catch(
            function(error) {
                console.log(error);
            }
        )
    }
}

async function addFolder() {
    folderCount += 1;
    await compilerAction({ type: "add_folder", path: currentPath + "Folder" + folderCount })
    await updateFileList();
}

async function renameEntry() {
    await compilerAction( {
        type: "rename_entry",
        from: currentPath + selectedEntry,
        to: currentPath + this.value,
    })
    selectedEntry = "";
    await updateFileList();
}

async function removeEntry() {
    const name_elem = this.parentNode.children[1];
    const name = name_elem.innerHTML;
    const type = name_elem.className;
    switch (type) {
        case "dir-name":
            await compilerAction({type: "remove_dir", path: currentPath + name})
            break;
        case "file-name":
            await compilerAction({type: "remove_file", path: currentPath + name})
            break;
    }
    await updateFileList();
}

function promptRename() {
    let name_elem = this.parentNode.children[1];
    selectedEntry = name_elem.innerHTML;
    let input = document.createElement("input");
    input.style.width = "6rem";
    input.setAttribute("type", "text");
    input.addEventListener("focusout", updateFileList, false);
    input.addEventListener("change", renameEntry, false);
    name_elem.replaceWith(input);
    input.focus();
}

async function downloadFile() {
    const name_elem = this.parentNode.children[1];
    const name = name_elem.innerHTML;
    const data = await compilerAction({type: "read_file", path: currentPath + name});
    const blob = new Blob([data]);
    const url = window.URL.createObjectURL(blob);

    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = name;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
}

async function updateFileList() {
    const list = [];

    for (let entry of JSON.parse(await compilerAction({type: "get_file_list", path: currentPath}))) {
        const [name, isFolder] = entry;
        const row = document.createElement("div");
        row.className = "dir-entry";

        const icon = document.createElement("span");
        icon.className = "material-symbols-outlined";

        const edit_button = document.createElement("button");
        edit_button.className = "rename-button";
        edit_button.innerHTML = '<span class="material-symbols-outlined">edit</span>'
        edit_button.addEventListener("click", promptRename, false);

        const remove_button = document.createElement("button");
        remove_button.className = "remove-button";
        remove_button.innerHTML = '<span class="material-symbols-outlined">delete</span>'
        remove_button.addEventListener("click", removeEntry, false);

        const text = document.createElement("div");
        text.innerHTML = name;

        if (isFolder) {
            icon.innerHTML = "folder_open";
            text.className = "dir-name";
            text.addEventListener("dblclick", visitDir, false);
        } else {

            icon.innerHTML = "description";
            text.className = "file-name";
        }

        row.appendChild(icon);
        row.appendChild(text);
        row.appendChild(edit_button);
        row.appendChild(remove_button);

        if (!isFolder) {
            const download_button = document.createElement("button");
            download_button.className = "download-button";
            download_button.innerHTML = '<span class="material-symbols-outlined">download</span>'
            download_button.addEventListener("click", downloadFile, false);
            row.appendChild(download_button);
        }

        list.push(row);
    }

    fileList.replaceChildren(...list);

    const folderName = currentPath.split("/").at(-2);
    if (folderName) {
        currentFolder.innerHTML = folderName;
    } else {
        currentFolder.innerHTML = "<em>root</em>"
    }
}

function visitDir() {
    currentPath += this.innerHTML;
    currentPath += "/";
    updateFileList();
}

function leaveDir() {
    if (currentPath.length > 1) {
        const trimmed = currentPath.slice(0, -1);
        currentPath = trimmed.slice(0, trimmed.lastIndexOf("/")+1);
        updateFileList();
    }
}

function addError(message) {
    errorPrompt.style.display = "block";
    errorLog.innerHTML += `<div class="issue">${message}</div>`;
}

function addWarning(message) {
    warningPrompt.style.display = "block";
    warningLog.innerHTML += `<div class="issue">${message}</div>`;
}


async function updateOutput(input) {
    // Clear the errors and warnings
    errorLog.innerText = "";
    errorPrompt.style.display = "none";
    warningLog.innerText = "";
    warningPrompt.style.display = "none";
    let start = new Date();
    try {
        switch (selector.value) {
            case "ast":
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";
                overleafButton.style.display = "none";

                debugEditor.session.setMode("");
                debugEditor.setValue(await compilerAction({ type: "ast", source: input }));
                debugEditor.getSession().selection.clearSelection()
                break;
            case "ast-debug":
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";
                overleafButton.style.display = "none";

                debugEditor.session.setMode("");
                debugEditor.setValue(await compilerAction({ type: "ast_debug", source: input }));
                debugEditor.getSession().selection.clearSelection();
                break;
            case "json-output":
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";
                overleafButton.style.display = "none";

                debugEditor.session.setMode("ace/mode/json");
                debugEditor.setValue(await compilerAction({ type: "json_output", source: input }));
                debugEditor.getSession().selection.clearSelection();
                break;
            case "transpile-other": {
                let { content, warnings, errors } = JSON.parse(await compilerAction({
                    type: "transpile",
                    source: input,
                    format: formatInput.value
                }));
                errors.forEach(addError);
                warnings.forEach(addWarning);

                overleafButton.style.display = "none";
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";


                debugEditor.session.setMode("");
                debugEditor.setValue(content);
                debugEditor.getSession().selection.clearSelection();
            }
                break;
            case "latex":
                let { content, warnings, errors } = JSON.parse(await compilerAction({
                    type: "transpile",
                    source: input,
                    format: "latex"
                }));
                errors.forEach(addError);
                warnings.forEach(addWarning);

                overleafButton.style.display = "inline-block";
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debugEditor.session.setMode("ace/mode/latex");
                debugEditor.setValue(content);
                debugEditor.getSession().selection.clearSelection();
                break;
            case "transpile":
            case "render-iframe":
            case "render": {
                let type = selector.value == "render" ? "transpile_no_document" : "transpile";
                let { content, warnings, errors } = JSON.parse(await compilerAction({
                    type,
                    source: input,
                    format: "html"
                }));
                errors.forEach(addError);
                warnings.forEach(addWarning);

                overleafButton.style.display = "none";

                if (selector.value === "transpile") {
                    debugEditor.container.style.display = "block";
                    renderIframe.style.display = "none";
                    render.style.display = "none";

                    debugEditor.session.setMode("ace/mode/html");
                    debugEditor.setValue(content);
                    debugEditor.getSession().selection.clearSelection()
                } else if (selector.value === "render-iframe") {
                    debugEditor.container.style.display = "none";
                    renderIframe.style.display = "block";
                    render.style.display = "none";

                    renderIframe.setAttribute("srcdoc", content);
                } else {
                    debugEditor.container.style.display = "none";
                    renderIframe.style.display = "none";
                    render.style.display = "block";

                    render.innerHTML = content;
                }
            }
                break;
        }
    } catch (error) {
        addError(error);
        // If we encounter a core or a parsing error there is no point
        // in displaying the output, so we hide those views.
        debugEditor.container.style.display = "none";
        render.style.display = "none";
    }
    let timeElapsed = new Date() - start;
    status.innerHTML = buttonContent(`Compiled in ${timeElapsed} ms`, "magic_button");
}

const overleafButton = document.getElementById("overleaf-button");
overleafButton.onclick = openInOverleaf;

function openInOverleaf() {
    let code = debugEditor.getValue();
    let url = "https://www.overleaf.com/docs"
    // post the code to overleaf
    let form = document.createElement("form");
    form.setAttribute("method", "post");
    form.setAttribute("action", url);
    form.setAttribute("target", "_blank");
    let hiddenField = document.createElement("input");
    hiddenField.setAttribute("type", "hidden");
    hiddenField.setAttribute("name", "snip");
    hiddenField.setAttribute("value", code);
    form.appendChild(hiddenField);
    document.body.appendChild(form);
    form.submit();
}
