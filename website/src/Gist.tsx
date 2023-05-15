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

async function getGistById(id: string): Promise<string | null> {
    let api_result: GistResp;
    try {
        let res = await fetch("https://api.github.com/gists/" + id);
        if (res.status !== 200) {
            return null;
        }
        let content = await res.text();
        api_result = JSON.parse(content) as GistResp;
    } catch {
        return null;
    }

    for (let [filename, file] of Object.entries(api_result.files)) {
        if (filename.endsWith(".mdm")) {
            return file.content;
        }
    }

    return null;
}

export default getGistById;
