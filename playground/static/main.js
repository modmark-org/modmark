import init, { ast, ast_debug, inspect_context, transpile, json_output } from "./pkg/web_bindings.js";

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
    editor.session.on("change", event => updateOutput(editor.getValue()));
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
            packageContent.innerText = inspect_context();
            selector.setAttribute("disabled", true);
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
