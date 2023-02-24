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

## Handling errors

Errors occur in all pieces of code, including your packages. As a part of the sandbox we build to run your packages in, we have built in some error handling.

Here are the main ideas:

* There are both **warnings** and **errors**, which are collected throughout the lifecycle of the compilation.
* **Warnings** are treated as just warnings, and will take the output of your package and try to use it, assuming that the document generation may still proceed.
* **Errors** are treated like errors, and the result of your package is ignored. If the output format has support for it, your module will be replaced by an error message that shows up in the document, at the place it occurred.
* After document compilation, the warnings and errors should somehow be displayed for you to see, separately from the document, since you may have a couple of warnings but still have a valid document.

We have tried to make it as simple as possible to generate **warnings**/**errors** if need be. As a part of `wasi` you have access to `stderr`, and the easiest and best way to generate **warnings**/**errors** is through that. Your code may also crash, and that will also be caught by our sandbox, but that should **never** be intentional.

* If you want to generate an **error**, you should print the error text to `stderr` and then exit early **without printing anything valid to `stdout`**. Printing nothing is considered invalid.
* If the sandbox sees that there is output in `stderr` and no valid output in `stdout`, it will treat **each line** in `stderr` as a separate error.
* If you want to generate a **warning**, you should print the warning text to `stderr` and then proceed to executing your code as normal, returning the normal output.
* If the sandbox sees that there is output in `stderr` but still a valid tree structure in `stdout`, it will treat **each line** in `stderr` as a separate warning
* If your module does crash, it will clearly be treated as an error but depending on the language you write in, the error text may not be as customizable as you could have hoped. It is
  ***not allowed to intentionally crash your module to emit errors***.

Under the hood, for each error an `[error]` module is created, and for each warning, a `[warning]` module is created. A native package takes care of them and takes the appropriate action. To have some sort of traceback, the modules take a `source` parameter, which is the name of the module that was invoked which caused the error, `target`, which is the target output format when the error occurred, and `input` which is the input that the module received when the error occurred. It is strongly advised to generate errors by `stderr` since these fields will be filled automatically, but if you want you may use these `[error]`/`[warning]` modules. When evaluated, `[warning]` modules do disappear, while `[error]` modules will check if there is a transform from `[__error]` to the desired output format. If there is, it will create such an `[__error]` module, copying the arguments, to allow the language package to show errors in-line.

If you are implementing the `[__error]` transform itself, it is very important that **it doesn't delegate to any other module which errors**. If an error occurs within the `[__error]` module, it will create an `[error]` module with `[__error]` as source, and the `[error]` module figures out that it shouldn't generate any more `[__error]` modules. But if the `[__error]` module generated some other module that crashed, the source of that error won't be `[__error]` and another `[__error]` will be created, possibly leading to infinite recursion. ***Only have simple behaviours within your `[__error]` handler to avoid infinite recursion. It is strongly suggested to not create any other nodes that may fail within the `[__error]` handler***
