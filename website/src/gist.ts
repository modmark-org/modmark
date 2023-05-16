type GistResp = {
    url: string,
    id: string,
    files: Record<string, GistFile>
}

type GistFile = {
    filename: string,
    raw_url: string,
    size: number,
    truncated: boolean,
    content: string
}

// This function tries to load a Gist with the given ID, and returns the appropriate entry point as a string, or if
// the Gist format is invalid, a string describing the error will be returned. If something goes wrong with the actual
// web requests, this function throws. If it finds a Gist, one of these cases will happen:
// * If it contains 0 files, the string "No files found in Gist" will be returned
// * If it contains 1 file ending in .mdm, that file will be returned
// * If it contains 1 file not ending in .mdm, the string "No .mdm file found" will be returned
// * If it contains multiple files, all files (but the one named "main.mdm") will be sent to otherFileHandler
// * If a "main.mdm" file is found, it is returned, otherwise the error message
//   "Gist contains multiple files but none named main.mdm" will be returned. Note that all files will still be sent
//   to otherFileHandler
async function getGistById(id: string, otherFileHandler: (path: string, bytes: Uint8Array) => void): Promise<string> {
    let api_result: GistResp;
    try {
        let res = await fetch("https://api.github.com/gists/" + id);
        if (res.status !== 200) {
            // noinspection ExceptionCaughtLocallyJS
            throw `Error fetching Gist with id ${id}: status code ${res.status}`;
        }
        let content = await res.text();
        api_result = JSON.parse(content) as GistResp;
    } catch (e) {
        return `Error loading Gist: ${e}`;
    }

    let entries = Object.entries(api_result.files);
    if (entries.length === 0) {
        return "No files found in Gist";
    }
    if (entries.length === 1) {
        let [filename, file] = entries[0];
        if (filename.endsWith(".mdm")) {
            return file.content;
        } else {
            return "No .mdm file found";
        }
    }

    let mainFile: string | null = null;
    let textEncoder = new TextEncoder();
    for (let [filename, file] of Object.entries(api_result.files)) {
        if (filename.toLowerCase() === "main.mdm") {
            mainFile = file.content;
        } else {
            let array = textEncoder.encode(file.content);
            otherFileHandler(filename, array);
        }
    }

    if (mainFile === null) {
        return "Gist contains multiple files but none named main.mdm";
    } else {
        return mainFile;
    }
}

export default getGistById;
