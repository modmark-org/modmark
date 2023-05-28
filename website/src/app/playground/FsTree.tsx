import { FiDownload, FiFolder, FiFolderPlus, FiUpload } from "react-icons/fi";
import { MdDriveFileRenameOutline, MdOutlineDelete } from "react-icons/md";
import { IoMdArrowBack, IoMdCheckmark, IoMdClose } from "react-icons/io";
import styled from "styled-components";
import { useEffect, useState } from "react";
import { FileUploader } from "react-drag-drop-files";
import Button from "../../components/Buttons";

const Container = styled.div`
  position: relative;
  background: #f3f3f3;
  overflow-y: auto;
  padding: 1rem;
  padding-top: 0;
  width: 30rem;
  box-sizing: border-box;
  border-right: solid 1px #00000013;

  .clickable {
    transition: color 0.2s ease-in-out;
  }

  .clickable:hover {
    background: #e7e7e7;
  }
`;

const EntryContainer = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;

  & input {
    max-width: 60%;
  }
`;

const Actions = styled.div`
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.2rem;
`;

const Action = styled.button`
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: none;
  padding: 0.2rem;
  font-size: 1.2rem;
  border-radius: 0.3rem;
  cursor: pointer;

  &:hover {
    background: #0000001c;
  }
`;

const Top = styled.div`
  padding: 0.5rem;
  padding-top: 1rem;
  box-sizing: border-box;
  padding-bottom: 0.5rem;
  position: sticky;
  display: flex;
  top: 0;
  background: #f3f3f3;
  flex-direction: column;
  gap: 0.5rem;
  overflow-x: hidden;
`;

const Upload = styled.div`
  position: relative;
  box-sizing: border-box;
  border: dashed 2px #e0e0e0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.5rem;
  width: 100%;
  border-radius: 0.5rem;
  cursor: pointer;

  &:hover {
    background: #00000013;
  }
`;

const Buttons = styled.div`
  display: flex;
  flex-direction: row;
  gap: 0.5rem;
  & > button {
    width: 50%;
  }
`;

const Entries = styled.div`
  margin-top: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
`;

const Name = styled.div`
  display: flex;
  max-width: 40ch;
  overflow: hidden;
  gap: 0.5rem;
`;

type EntryProps = {
  name: string;
  isDir: boolean;
  onRemove: () => void;
  onDownload: () => void;
  onRename: (to: string) => void;
  onMove: () => void;
};

function Entry({ name, isDir, onRemove, onDownload, onRename, onMove }: EntryProps) {
  const [renaming, setRenaming] = useState(false);
  const [newName, setNewName] = useState(name);

  const cancelRename = () => {
    setNewName(name);
    setRenaming(false);
  };

  const rename = () => {
    onRename(newName);
    setRenaming(false);
  };

  // If you click on then name of a folder, move to that directory
  const nameClick = () => {
    if (isDir) {
      onMove();
    }
  };

  return (
    <EntryContainer>
      {renaming ? (
        <>
          <Name>
            {isDir && <FiFolder />}
            <input
              autoFocus
              type="text"
              onKeyDown={(e) => {
                if (e.key === "Enter") rename();
                else if (e.key === "Escape") cancelRename();
              }}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
          </Name>

          <Actions>
            <Action onClick={rename}>
              <IoMdCheckmark />
            </Action>
            <Action onClick={cancelRename}>
              <IoMdClose />
            </Action>
          </Actions>
        </>
      ) : (
        <>
          <Name style={isDir ? { cursor: "pointer" } : {}} onClick={nameClick}>
            {isDir && <FiFolder />}
            <span className={isDir ? "clickable" : ""}>{name}</span>
          </Name>

          <Actions>
            <Action onClick={() => setRenaming(true)}>
              {" "}
              <MdDriveFileRenameOutline />
            </Action>
            {!isDir && (
              <Action onClick={onDownload}>
                <FiDownload />
              </Action>
            )}
            <Action onClick={onRemove}>
              <MdOutlineDelete />
            </Action>
          </Actions>
        </>
      )}
    </EntryContainer>
  );
}

export type FsTreeProps = {
  listFiles: (path: string) => Promise<void | any>;
  addFile: (path: string, bytes: Uint8Array) => Promise<void>;
  readFile: (path: string) => Promise<Uint8Array | null>;
  renameEntry: (from: string, to: string) => Promise<void>;
  removeFile: (path: string) => Promise<void>;
  removeFolder: (path: string) => Promise<void>;
  addFolder: (path: string) => Promise<void>;
  folderCounter: number;
  incFolderCounter: () => void;
};

export default function FsTree({
  listFiles,
  addFile,
  readFile,
  removeFolder,
  removeFile,
  renameEntry,
  addFolder,
  incFolderCounter,
  folderCounter,
}: FsTreeProps) {
  const [workingDir, setWorkingDir] = useState("/");
  const [entries, setEntries] = useState<[string, boolean][]>([]);

  const updateList = () => {
    listFiles(workingDir).then((files) => {
      if (files !== null) {
        setEntries(files);
      }
    });
  };

  useEffect(updateList, [workingDir, listFiles]);

  const handleUpload = (files: File[]) => {
    for (const file of files) {
      // convert the file to a Uint8Array
      let reader = new FileReader();
      reader.readAsArrayBuffer(file);
      reader.onload = async () => {
        let arrayBuffer = reader.result as ArrayBuffer;
        let bytes = new Uint8Array(arrayBuffer);

        await addFile(workingDir + file.name, bytes);
        updateList();
      };
    }
  };

  const handleRemove = (name: string, isDir: boolean) => {
    if (isDir) {
      removeFolder(workingDir + name).then(updateList);
    } else {
      removeFile(workingDir + name).then(updateList);
    }
  };

  const handleRename = (from: string, to: string) => {
    renameEntry(workingDir + from, workingDir + to).then(updateList);
  };

  const handleBack = () => {
    // go back to the parent directory
    const parts = workingDir.split("/");
    console.log(parts);
    parts.pop();
    parts.pop();
    setWorkingDir(parts.join("/") + "/");
  };

  const handleDownload = (name: string) => {
    readFile(workingDir + name).then((data) => {
      if (data === null) {
        return;
      }

      const blob = new Blob([data]);
      const url = window.URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = name;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
    });
  };

  return (
    <Container>
      <Top>
        <strong>Current directory: {workingDir}</strong>
        <Buttons>
          <Button onClick={handleBack} disabled={workingDir === "/"}>
            <IoMdArrowBack /> Back
          </Button>
          <Button
            onClick={() => {
              addFolder(workingDir + "Folder" + folderCounter);
              incFolderCounter();
              updateList();
            }}>
            <FiFolderPlus /> New folder
          </Button>
        </Buttons>
        <FileUploader
          multiple
          class="test"
          handleChange={handleUpload}
          label="Upload or drag and drop a file here">
          <Upload>
            <FiUpload size={30} /> Click or drag and drop to upload a file
          </Upload>
        </FileUploader>
      </Top>
      <Entries>
        {entries.map(([name, isDir]) => (
          <Entry
            isDir={isDir}
            onMove={() => setWorkingDir(workingDir + name + "/")}
            onRemove={() => handleRemove(name, isDir)}
            onDownload={() => handleDownload(name)}
            onRename={(to) => handleRename(name, to)}
            name={name}
            key={name}
          />
        ))}
      </Entries>
    </Container>
  );
}
