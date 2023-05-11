import styled from "styled-components"
import Editor from '@monaco-editor/react';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';

type Monaco = typeof monaco;

export type Mode =
    "ast"
    | "ast-debug"
    | "json-output"
    | "html"
    | "render-html"
    | "render-html-iframe"
    | "latex"
    | "transpile-other";

export type PreviewProps = {
    content: string,
    mode: Mode,
    valid: boolean,
}

const PreviewContainer = styled.div<{ valid: boolean }>`
  width: 100%;
  height: 100%;
  opacity: ${props => props.valid ? 1 : 0.4};
`;

const Iframe = styled.iframe`
  border: none;
  width: 100%;
  height: 100%;
`;

const HtmlPreview = styled.div`
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  padding: 1rem;
  overflow: auto;
`

function CodeEditor({content, language}: { content: string, language: string }) {
    return <Editor
        height="100%"
        language={language}
        value={content}
        options={{readOnly: true}}
    />
}

export function Preview({content, mode, valid}: PreviewProps) {

    let main;

    if (mode === "ast") {
        main = <CodeEditor content={content} language="text"/>
    } else if (mode === "ast-debug") {
        main = <CodeEditor content={content} language="rust"/>
    } else if (mode === "json-output") {
        main = <CodeEditor content={content} language="json"/>
    } else if (mode === "html") {
        main = <CodeEditor content={content} language="html"/>
    } else if (mode === "render-html") {
        main = <HtmlPreview dangerouslySetInnerHTML={{__html: content}}/>
    } else if (mode === "render-html-iframe") {
        main = <Iframe srcDoc={content}/>
    } else if (mode === "latex") {
        main = <CodeEditor content={content} language="text"/>
    } else if (mode === "transpile-other") {
        main = <CodeEditor content={content} language="text"/>
    }

    return <PreviewContainer valid={valid}>
        {main}
    </PreviewContainer>
}
