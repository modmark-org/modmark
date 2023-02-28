import init, { ast, ast_debug, package_info, transpile, json_output } from "./pkg/web_bindings.js";

let view = "editor";
const editorView = document.getElementById("editor-view");

// Setup the editor
const editorOptions = {
    fontFamily: "IBM Plex Mono",
    fontSize: "12pt"
};

let editor = ace.edit("editor");
editor.setOptions(editorOptions);
editor.session.setUseWrapMode(true);

//  EWarnings and errors
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


init().then(() => {
    editor.session.on("change", (_event) => handleChange());
    selector.onchange = () => updateOutput(editor.getValue());
    updateOutput(editor.getValue());
});

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
            packageContent.innerText =
                selector.setAttribute("disabled", true);
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


function loadPackageInfo() {
    const info = JSON.parse(package_info());

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


function updateOutput(input) {
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

                debugEditor.session.setMode("");
                debugEditor.setValue(ast(input));
                debugEditor.getSession().selection.clearSelection()
                break;
            case "ast-debug":
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debugEditor.session.setMode("");
                debugEditor.setValue(ast_debug(input));
                debugEditor.getSession().selection.clearSelection()
                break;
            case "json-output":
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debugEditor.session.setMode("ace/mode/json");
                debugEditor.setValue(json_output(input));
                debugEditor.getSession().selection.clearSelection()
                break;
            case "transpile":
            case "render-iframe":
            case "render": {
                let { content, warnings, errors } = JSON.parse(transpile(input));
                console.log({ warnings: warnings, errors: errors });
                errors.forEach(addError);
                warnings.forEach(addWarning);

                if (selector.value == "transpile") {
                    debugEditor.container.style.display = "block";
                    renderIframe.style.display = "none";
                    render.style.display = "none";

                    debugEditor.session.setMode("ace/mode/html");
                    debugEditor.setValue(content);
                    debugEditor.getSession().selection.clearSelection()

                } else if (selector.value == "render-iframe") {
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
            } break;
        }
    } catch (error) {
        addError(error);
        // If we encounter a core or a parsing error there is no point
        // in displaying the output so we hide those views.
        debugEditor.container.style.display = "none";
        render.style.display = "none";
    }
    let deltaT = new Date() - start;
    status.innerHTML = buttonContent(`Compiled in ${deltaT} ms`, "magic_button");
}

