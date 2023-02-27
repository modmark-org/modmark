import init, { ast, ast_debug, package_info, transpile, json_output } from "./pkg/web_bindings.js";

let view = "editor";
const editorView = document.getElementById("editor-view");

// Setup the editor
const editorOptions = {
    fontFamily: "IBM Plex Mono",
    fontSize: "14pt"
};

let editor = ace.edit("editor");
editor.setOptions(editorOptions);
editor.session.setUseWrapMode(true);

//  Editor/preview view
const errorLog = document.getElementById("error-log");
const debugEditor = ace.edit("debug-editor");
debugEditor.setOptions(editorOptions);
debugEditor.setReadOnly(true);
debugEditor.setHighlightActiveLine(false);
debugEditor.container.style.background = "#f2f2f2"
const render = document.getElementById("render");
const renderIframe = document.getElementById("render-iframe");
const errorPrompt = document.getElementById("error-prompt");
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


function updateOutput(input) {
    // Clear the errors
    errorLog.innerText = "";
    errorPrompt.style.display = "none";
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
                debugEditor.container.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debugEditor.session.setMode("ace/mode/html");
                debugEditor.setValue(transpile(input));
                debugEditor.getSession().selection.clearSelection()
                break;
            case "render-iframe":
                debugEditor.container.style.display = "none";
                renderIframe.style.display = "block";
                render.style.display = "none";

                renderIframe.setAttribute("srcdoc", transpile(input));
                break;
            case "render":
                debugEditor.container.style.display = "none";
                renderIframe.style.display = "none";
                render.style.display = "block";

                render.innerHTML = transpile(input);
                break;
        }
    } catch (error) {
        errorPrompt.style.display = "block";
        errorLog.innerHTML = error;

        debugEditor.container.style.display = "none";
        render.style.display = "none";

    }
    let deltaT = Math.abs(start - (new Date()));
    status.innerHTML = buttonContent(`Compiled in ${deltaT} ms`, "magic_button");
}
