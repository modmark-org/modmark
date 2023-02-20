# Packages

Hey there!

In this folder, you can find a bunch of useful packages that come with the compiler. But, what are packages, you ask? Well, they are programs that add support for transforming **module elements** (like `[math]` and `[table]`) as well as **parent elements** (like `**bold text**` and the document template) into a output format.

Packages work similarly to command-line programs: they receive a part of theto document in json format through stdin, modify it, and send the result back via stdout. Packages are loaded as `.wasm` files, which means they can be written in any programming language that supports WebAssembly (WASM) and the WebAssembly System Interface (WASI).

For example, if you want to create your own package in Rust, compile it to `--target wasm32-wasi`.


# Writing a package of your own
If you wish to write your own package you need to handle two types of request that are specified as command-line arguments to your program.

## Manifest
When your program is called with the arguments `$ ./my_program manifest` it expects to recieve a json object (sent via stdout) that explains how your package works and what transforms it supports. A manifest might look something like this:
```json
{
    "name": "Name of your package",
    "version": "0.1",
    "description": "A short description that explains the function of your package",
    "transforms": [
            {
                "from": "foo",
                "to": ["html", "tex"],
                "arguments": [
                    {"name": "a_argument", "description": "This is the description for the argument 'a_argument'"},
                    {"name": "another_argument", "default": "hello", "description": "..."}
                ],
            },
            {
                "from": "bar",
                "arguments": [],
            }
    ]
}
```

## Transforming an element
When the program is called with `$ ./my_program transform <element_name> <output_format>` you will need to transform an element into the youdesired output format.

You will be sent the element as json object via stdout and you respond by sending a json list of objects representing elements back.

If you want to see some examples of what this json format looks like, you can check out the subdirectories or visit the online playground and select "JSON output".