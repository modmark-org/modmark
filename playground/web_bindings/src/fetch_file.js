export async function fetch_file(url) {
    const response = await fetch(url, {mode: "no-cors"});
    const reader = response.body.getReader();
    let array = new Uint8Array(0);
    while (true) {
        const {done, value} = await reader.read();
        let new_array = new Uint8Array(array.length + value.length);
        new_array.set(array, 0);
        new_array.set(value, array.length);
        array = new_array;
        if (done) {
            // Do something with last chunk of data then exit reader
            return array;
        }
        // Otherwise do something here to process current chunk
    }
}
