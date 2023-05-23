import styled from "styled-components";
import PackageDocs from "./PackageDocs";
import { CompilationException, Compiler, handleException, PackageInfo } from "./compilerTypes";
import { useEffect, useMemo, useState } from "react";
import * as Comlink from "comlink";
import { Link } from "react-router-dom";
import { FileUploader } from "react-drag-drop-files";
import { FiUpload } from "react-icons/fi";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
`;

const Hero = styled.div`
  padding: 1rem;
  padding-top: 2rem;
  padding-bottom: 2rem;
  background: #f1f1f1;

  & > div {
    max-width: 800px;
    margin-left: auto;
    margin-right: auto;
  }

  & > div > p {
    max-width: 40ch;
    opacity: 0.7;
  }
`;

const Upload = styled.div`
  position: relative;
  box-sizing: border-box;
  border: dashed 2px #7392b7;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.5rem;
  width: 100%;
  max-width: 350px;
  border-radius: 0.5rem;
  cursor: pointer;

  &:hover {
    background: #00000013;
  }
`;

const MainWrapper = styled.div`
  width: 100%;
  background-color: white;
`;

const Main = styled.main`
  max-width: 800px;
  margin-left: auto;
  margin-right: auto;
`;

const Errors = styled.div`
  background: #c1666b;
  border-radius: 0.5rem;
  color: white;
  padding: 1rem;
  margin-top: 1rem;

  & pre {
    opacity: 0.5;
  }
`;

export default function PackageDocsPage() {
  const [loaded, setLoaded] = useState(false);
  const [imports, setImports] = useState<string>("");
  const [error, setError] = useState<null | CompilationException>(null);
  const [packages, setPackages] = useState<PackageInfo[]>([]);

  // Init the compiler
  const compiler: Compiler = useMemo(
    () => Comlink.wrap(new Worker(new URL("./worker.js", import.meta.url))),
    [],
  );

  useEffect(() => {
    let init = async () => {
      await compiler.init();
    };
    init()
      .then(() => {
        setLoaded(true);
        compiler.package_info().then((pkgs) => {
          if (pkgs !== null) {
            setPackages(pkgs);
          }
        });
      })
      .catch(console.error);
  }, []);

  // Load a package from a file and store in
  // the virtual file system
  const loadPackage = (file: File) => {
    // convert the file to a Uint8Array
    let reader = new FileReader();
    reader.readAsArrayBuffer(file);
    reader.onload = async () => {
      // If we are loading our own files, make sure to clean the context
      // to avoid any name collisions.
      await compiler.blank_context();

      let arrayBuffer = reader.result as ArrayBuffer;
      let bytes = new Uint8Array(arrayBuffer);

      await compiler.add_file(file.name, bytes);

      setImports("[config]\n" + "import " + file.name + "\n");
    };
  };

  // Configure context based on the provided import statements
  const configure = (time: number) => {
    setTimeout(() => {
      compiler
        .configure_from_source(imports)
        .then((complete) => {
          if (!complete) {
            // attempt to configure again if we are not done yet
            configure(time * 2);
          } else {
            setError(null);
            compiler.package_info().then((pkgs) => {
              if (pkgs !== null) {
                setPackages(pkgs);
              }
            });
          }
        })
        .catch((err) => {
          setError(handleException(err));
        });
    }, time);
  };

  useEffect(() => configure(10), [imports]);

  return (
    <Container>
      <Hero>
        <div>
          <Link to="../">
            <img src="./logo.svg" alt="Logo" width="80" />
          </Link>
          <h1>Package documentation</h1>
          <p>
            Read documentation for the standard library, or drag and drop one of your own packages.
          </p>
          <FileUploader
            handleChange={loadPackage}
            label="Load package, or drop a .WASM file here"
            types={["WASM"]}>
            <Upload>
              <FiUpload size={30} /> Click or drag and drop to upload a file
            </Upload>
          </FileUploader>
          {error !== null && (
            <Errors>
              {error.type === "compilationError" &&
                error.data.map((e, i) => (
                  <div key={i}>
                    <p>{e.message}</p>
                    <pre>{e.raw}</pre>
                  </div>
                ))}
              {error.type === "parsingError" && (
                <div>
                  <p>{error.data.message}</p>
                  <pre>{error.data.raw}</pre>
                </div>
              )}
            </Errors>
          )}
        </div>
      </Hero>
      <MainWrapper>
        <Main>{loaded ? <PackageDocs packages={packages} /> : <p>Loading compiler...</p>}</Main>
      </MainWrapper>
    </Container>
  );
}
