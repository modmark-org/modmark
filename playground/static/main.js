import init, { ast, ast_debug, package_info, transpile, json_output } from "./pkg/web_bindings.js";

let view = "editor";
const editorView = document.getElementById("editor-view");

// Setup the editor
let editor = ace.edit("editor");
editor.setOptions({
    fontFamily: "IBM Plex Mono",
    fontSize: "14pt"
});

editor.session.setUseWrapMode(true);

//  Editor/preview view
const errorLog = document.getElementById("error-log");
const debug = document.getElementById("debug");
const render = document.getElementById("render");
const renderIframe = document.getElementById("render-iframe");
const errorPrompt = document.getElementById("error-prompt");

// Package view
const packageView = document.getElementById("package-view");
const packageContent = document.getElementById("package-content");

// Menu options
const selector = document.getElementById("selector");
const viewToggle = document.getElementById("view-toggle");
const leftMenu = document.getElementById("left-menu");

viewToggle.onclick = toggleView;



init().then(() => {
    editor.session.on("change", (event) => updateOutput(editor.getValue()));
    selector.onchange = () => updateOutput(editor.getValue());
    const regex = /pr-preview\/pr-(\d+)/;
    const match = document.location.href.match(regex);
    if (match !== null) {
        const branch = match[1]
        const button = document.createElement("button");
        button.innerHTML = buttonContent("Previewing #" + branch, "alt_route");
        button.onclick = () => window.location = `https://github.com/modmark-org/modmark/pull/${branch}`;
        leftMenu.appendChild(button);
    }
});



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
            <span class="default">${arg.default ? 'default = \"' + arg.default + "\"" : 'required'}"</span>
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

    try {
        switch (selector.value) {
            case "ast":
                debug.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debug.innerText = ast(input);
                break;
            case "ast-debug":
                debug.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debug.innerText = ast_debug(input);
                break;
            case "json-output":
                debug.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debug.innerText = json_output(input);
                break;
            case "transpile":
                debug.style.display = "block";
                renderIframe.style.display = "none";
                render.style.display = "none";

                debug.innerText = transpile(input);
                break;
            case "render-iframe":
                debug.style.display = "none";
                renderIframe.style.display = "block";
                render.style.display = "none";

                renderIframe.setAttribute("srcdoc", transpile(input));
                break;
            case "render":
                debug.style.display = "none";
                renderIframe.style.display = "none";
                render.style.display = "block";

                render.innerHTML = transpile(input);
                break;
        }
    } catch (error) {
        errorPrompt.style.display = "block";
        errorLog.innerHTML = error;

        debug.style.display = "none";
        render.style.display = "none";

    }
}
