import * as Comlink from "comlink";
import styled from 'styled-components'
import {useEffect, useMemo, useRef, useState} from "react";
import {Button, Input, Select} from "./Buttons";
import Editor from '@monaco-editor/react';
import {editor} from 'monaco-editor';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';
import {Link} from "react-router-dom";
import welcomeMessage from "./welcomeMessage";
import {Mode, Preview} from "./Preview";
import {FiBook, FiClock, FiFolder, FiPackage} from "react-icons/fi";
import {MdOutlineAutoAwesome, MdOutlineDownloading, MdOutlineKeyboardAlt} from "react-icons/md";
import FsTree from "./FsTree";
import PackageDocs from "./PackageDocs";
import {CompilationResult, Compiler, handleException, PackageInfo} from "./compilerTypes";
import Guide from "./Guide";


type Monaco = typeof monaco;

const Container = styled.div`
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  height: calc(100vh - 3rem);
  box-sizing: border-box;
`;

const Menu = styled.nav`
  width: 100%;
  padding-left: 1rem;
  padding-right: 1rem;
  height: 4rem;
  box-sizing: border-box;
  background: #f1f1f1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-direction: row;
  border-bottom: solid 1px #00000013;

  & > div {
    display: flex;
    align-items: center;
    flex-direction: row;
    gap: 1rem;
  }
`;

const Main = styled.main`
  position: relative;
  display: flex;
  height: calc(100% - 4rem);

  & > * {
    flex-grow: 1;
    flex-shrink: 1;
  }
`;

const View = styled.div`
  position: relative;
  width: 50%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
`;

const EditorContainer = styled.div`
  display: flex;
  width: 50%;
  box-sizing: border-box;
  border-right: 1px solid #00000013;
`

const Logo = styled.div`
  display: flex;
  align-items: center;
  gap: 0.5rem;

  & img {
    height: 2.5rem;
    width: 2.5rem;
  }

  & span {
    color: #1a1a1a;
    font-size: 0.9rem;
    font-weight: bold;
  }
`;

const Status = styled.div`
  display: flex;
  align-items: center;
  gap: 0.5rem;
  right: 1rem;
  bottom: 1rem;
  box-sizing: border-box;
  padding: 0.5rem 1rem 0.5rem 1rem;
  font-size: 0.9rem;
  background: #f7f7f7;

  & strong {
    margin-right: 1rem;
  }

  & svg {
    position: relative;
    top: 2px;
  }
`;

type Status =
    { type: "time", timeStart: Date, timeEnd: Date }
    | { type: "typing" }
    | { type: "compiling" }
    | { type: "loading" };

const COMPILE_INTERVAL = 300;

function Playground() {
    const [content, setContent] = useState("");
    const [showFiles, setShowFiles] = useState(false);
    const [activeView, setActiveView] = useState<"preview" | "docs" | "guide">("preview");
    const [packages, setPackages] = useState<PackageInfo[]>([]);
    const [loadingPackage, setLoadingPackage] = useState(false);
    const [selectedMode, setSelectedMode] = useState<Mode>("render-html");
    const [activeMode, setActiveMode] = useState<Mode>("render-html");
    const [otherOutputFormat, setOtherOutputFormat] = useState("");
    const [compilerLoaded, setCompilerLoaded] = useState(false);
    const [_compileTimeoutId, setCompileTimoutId] = useState<number | null>(null);
    const [status, setStatus] = useState<Status | null>({type: "loading"});
    // for the file system to avoid name collisions when creating new folders
    const [folderCount, setFolderCount] = useState(0);

    const [errors, setErrors] = useState<string[]>([]);
    const [warnings, setWarnings] = useState<string[]>([]);
    const [validPreview, setValidPreview] = useState(true);

    const [compilationCounter, setCompilationCounter] = useState(0);

    // Init the compiler
    const compiler: Compiler = useMemo(() => Comlink.wrap(new Worker(new URL('./worker.js', import.meta.url))), []);

    useEffect(() => {
        compiler.init()
            .then(() => setCompilerLoaded(true))
            .catch(console.error);
    }, []);

    const compile = (input: string, mode: Mode, instant: boolean) => {
        if (!compilerLoaded) {
            return;
        }

        const compile_helper = () => {
            setStatus({type: "compiling"});
            let start = new Date();
            let output;
            if (mode === "ast") {
                output = compiler.ast(input);
            } else if (mode === "ast-debug") {
                output = compiler.ast_debug(input);
            } else if (mode === "json-output") {
                output = compiler.json_output(input);
            } else if (mode === "html") {
                output = compiler.transpile(input, "html");
            } else if (mode === "render-html") {
                output = compiler.transpile_no_document(input, "html");
            } else if (mode === "render-html-iframe") {
                output = compiler.transpile(input, "html");
            } else if (mode === "latex") {
                output = compiler.transpile(input, "latex")
            } else if (mode === "transpile-other") {
                output = compiler.transpile(input, otherOutputFormat);
            }

            output?.then((result => {
                setLoadingPackage(false);
                let end = new Date();
                setStatus({type: "time", timeStart: start, timeEnd: end});
                if (result === null) {
                    setActiveMode(mode);
                    setValidPreview(false);
                    return;
                }

                // Update the list of packages too
                compiler.package_info().then((packages) => setPackages(packages ?? []));

                // ast and json output can't produce transpilation errors so we just use the input as is
                if (mode === "ast" || mode === "ast-debug" || mode === "json-output") {
                    setContent((result as string | null) ?? "");
                    setActiveMode(mode);
                    setErrors([]);
                    setWarnings([]);
                    setValidPreview(true);
                    return;
                }

                let {content, warnings, errors} = result as CompilationResult;
                setContent(content);
                setWarnings(warnings);
                setErrors(errors);
                setActiveMode(mode);
                setValidPreview(true);
            })).catch(
                // Log any unrecoverable compilation errors and handle packages that are loading
                (e) => {
                    let error = handleException(e);
                    let loggedErrors: string[] = [];

                    if (error.type === "compilationError") {
                        setLoadingPackage(false);
                        loggedErrors = error.data.map((e) => `<p>${e.message}</p><pre>${e.raw}</pre>`);
                    } else if (error.type === "parsingError") {
                        setLoadingPackage(false);
                        loggedErrors.push(`<p>${error.data.message}</p><pre>${error.data.raw}</pre>`)
                    } else if (error.type === "noResult") {
                        // If we are trying to load/download a package, update the status
                        setErrors([]);
                        setLoadingPackage(true);

                        // attempt to recompile after a short while
                        setTimeout(compile_helper, 200);
                    }
                    setErrors(loggedErrors);
                    setWarnings([]);
                    setActiveMode(mode);
                    setValidPreview(false); // invalidate the current preview
                })
        }
        setStatus({type: "typing"});
        setCompileTimoutId(oldId => {
            oldId && clearTimeout(oldId);
            return setTimeout(compile_helper, instant ? 0 : COMPILE_INTERVAL) as unknown as number;
        });
    }

    // save a reference to the monaco editor
    const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
    const handleEditorDidMount = (editor: editor.IStandaloneCodeEditor, _monaco: Monaco) => {
        editorRef.current = editor;
        editor.setValue(localStorage.getItem("input") ?? welcomeMessage);
    }

    const handleEditorChange = (value: string | undefined) => {
        if (value === undefined) {
            return;
        }
        localStorage.setItem("input", value);
        compile(value, activeMode, false);
    }

    const handleModeChange = (mode: Mode) => {
        setSelectedMode(mode);
        compile(editorRef.current?.getValue() ?? "", mode, true);
    }

    // compile the document once both the editor and compiler is ready
    useEffect(() => {
        if (compilerLoaded && editorRef.current) {
            compile(editorRef.current.getValue(), selectedMode, true);
        }
    }, [compilerLoaded, editorRef.current]);

    // recompile the document if using the "other output format" mode and a new output format was provided
    useEffect(() => {
        compile(editorRef.current?.getValue() ?? "", selectedMode, false);
    }, [otherOutputFormat]);

    const statusElem = <Status>
        <strong>Preview</strong>
        {status?.type === "time" &&
            <span><MdOutlineAutoAwesome/> Compiled in {status.timeEnd.getTime() - status.timeStart.getTime()}ms</span>}
        {status?.type === "typing" && <span><MdOutlineKeyboardAlt/> Typing...</span>}
        {status?.type === "compiling" && <span><FiClock/> Compiling...</span>}
        {status?.type === "loading" && <span><MdOutlineDownloading/>Loading compiler...</span>}
    </Status>;

    return (
        <Container>
            <Menu>
                <div>
                    <Logo>
                        <Link to="../">
                            <img src="./logo.svg" alt="logo"/>
                        </Link>
                        <span>ModMark<br/>Playground</span>
                    </Logo>
                    <Button active={showFiles} onClick={() => setShowFiles((showFiles) => !showFiles)}><FiFolder/> Files
                    </Button>
                    <Select value={selectedMode} onChange={(e) => handleModeChange(e.target.value as Mode)}>
                        <option value="ast">Abstract syntax tree</option>
                        <option value="ast-debug">Debug AST</option>
                        <option value="json-output">JSON tree</option>
                        <option value="html">Raw HTML</option>
                        <option value="transpile-other">Other output format</option>
                        <option value="render-html">Rendered HTML</option>
                        <option value="render-html-iframe">Rendered HTML (iframe)</option>
                        <option value="latex">LaTeX</option>
                    </Select>
                    {
                        selectedMode === "transpile-other" &&
                        <Input type="text" placeholder="Output format" value={otherOutputFormat}
                               onChange={(e) => setOtherOutputFormat(e.target.value)}
                        />
                    }
                    {
                        selectedMode === "latex" &&
                        <form method="POST" action="https://www.overleaf.com/docs" target="_blank">
                            <input readOnly value={content} name="snip" style={{display: "none"}}/>
                            <Input type="submit" value="Open in Overleaf"/>
                        </form>

                    }
                </div>
                <div>
                    <Button
                        active={activeView === "guide"}
                        onClick={() => setActiveView(activeView === "guide" ? "preview" : "guide")}
                    >
                        <FiBook/> Guide
                    </Button>

                    <Button
                        active={activeView === "docs"}
                        onClick={() => setActiveView(activeView === "docs" ? "preview" : "docs")}
                    >
                        <FiPackage/> Package docs
                    </Button>
                </div>
            </Menu>
            <Main>
                {
                    showFiles && <FsTree
                        folderCounter={folderCount}
                        incFolderCounter={() => setFolderCount((c) => c + 1)}
                        addFolder={compiler.add_folder}
                        renameEntry={compiler.rename_entry}
                        listFiles={compiler.get_file_list}
                        addFile={compiler.add_file}
                        removeFile={compiler.remove_file}
                        removeFolder={compiler.remove_folder}
                        readFile={compiler.read_file}
                    />
                }

                <EditorContainer>
                    <Editor
                        height="100%"
                        options={{minimap: {enabled: false}, quickSuggestions: false, wordWrap: "on"}}
                        defaultValue="// some comment"
                        onMount={handleEditorDidMount}
                        onChange={handleEditorChange}
                    />
                </EditorContainer>
                <View>
                    {activeView === "docs" &&
                        <div style={{
                            maxWidth: 800,
                            paddingBottom: "3rem",
                            height: "100%",
                            overflow: "auto",
                            width: "100%",
                            marginLeft: "auto",
                            marginRight: "auto"
                        }}>
                            <PackageDocs packages={packages}/>
                        </div>
                    }
                    {
                        activeView === "guide" &&
                        <Guide/>
                    }
                    {
                        activeView === "preview" && <>
                            <IssuesReport warnings={warnings} errors={errors}/>
                            {loadingPackage &&
                                <LoadingPackage><FiPackage size="20"/> Attempting to load package ...</LoadingPackage>}
                            {statusElem}
                            <Preview content={content} mode={activeMode} valid={validPreview}/>
                        </>
                    }
                </View>
            </Main>
        </Container>
    )
}

export default Playground;


const LoadingPackage = styled.div`
  background: #ececec;
  padding: 1rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
`;


const IssuesContainer = styled.div`
  width: 100%;
`;

const IssuesBox = styled.div`
  box-sizing: border-box;
  padding: 1rem;
  width: 100%;
`;

const ErrorContainer = styled(IssuesBox)`
  background: #fceced;

  & strong {
    color: #de455a;
  }
`;

const WarningContainer = styled(IssuesBox)`
  background: #fef7ea;

  & strong {
    color: #d69c15;
  }
`;

const Error = styled.div`
  border-top: solid 1px #e9cdc4;
  margin-top: 1rem;
  padding: 0.5rem;

  & > pre {
    opacity: 0.6;
  }
`

const Warning = styled.div`
  margin-top: 0.5rem;
  padding: 0.5rem;
  border-top: solid 1px #efddb9;

  & > pre {
    opacity: 0.6;
  }
`

// display warnings and errors
function IssuesReport({warnings, errors}: { warnings: string[], errors: string[] }) {
    const errorsElem = errors.map((error, i) => <Error key={i} dangerouslySetInnerHTML={{__html: error}}/>);

    const warningsElem = warnings.map((warning, i) => <Warning key={i} dangerouslySetInnerHTML={{__html: warning}}/>);

    return <IssuesContainer>
        {
            errorsElem.length > 0 &&
            <ErrorContainer>
                <strong>Errors</strong>
                {errorsElem}
            </ErrorContainer>
        }
        {
            warningsElem.length > 0 &&
            <WarningContainer>
                <strong>Warnings</strong>
                {warningsElem}
            </WarningContainer>
        }

    </IssuesContainer>
}