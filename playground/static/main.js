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

// Menu options
const selector = document.getElementById("selector");
const viewToggle = document.getElementById("view-toggle");
const leftMenu = document.getElementById("left-menu");
const formatInput = document.getElementById("format-input");

// Set to "render html" by default
selector.value = "render";

if (selector.value === "transpile-other") {
    formatInput.style.display = "block";
} else {
    formatInput.style.display = "none";
}

viewToggle.onclick = toggleView;

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

    const createTransformList = (transform) => {
        let args = transform.arguments.map((arg) => `<li><div>
            <strong class="name">${arg.name}</strong>
            <span class="default">${arg.default ? 'default = \"' + arg.default + "\"" : 'required'}</span>
            <span class="description" > ${arg.description}</span>
        </div></li> `).join("\n");

        return `<div>
            <code class="from">${transform.from}</code>
            <span class="to">${transform.to.join(" ")}</span>
            <ul class="arguments">${args}</ul>
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
                let { content, warnings, errors } = JSON.parse(await compilerAction({ type: "transpile", source: input, format: formatInput.value }));
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
                let { content, warnings, errors } = JSON.parse(await compilerAction({ type: "transpile", source: input, format: "latex" }));
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
                let { content, warnings, errors } = JSON.parse(await compilerAction({ type: "transpile", source: input, format: "html" }));
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
