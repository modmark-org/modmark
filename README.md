# ModMark

Welcome to the main ModMark repo! This is the home of the ModMark modular markup language.

## Usage

To invoke the ModMark compiler, you can run

> cargo run -- compile in.mdm out.html

This will compile the input document `in.mdm` transforming it to HTML, and saving the output as `out.html`. In addition to the host platform target, the `wasm32-wasi` target is needed; `rustup target add wasm32-wasi`. See more information about the CLI tool at [cli/readme](cli/README.md).

## Overview

The ModMark language is a markup language which is highly modular - you may use different packages to augment the language with new modules, adding completely new functionality. The packages themselves can be written in a variety of different programming languages and may be included in your project to be able to parse whole new expressions.

Here is an example ModMark document:

```
# Welcome to **ModMark**
This is **ModMark**, a modular markup language. The language itself is
small and has few built-in features, but by using //packages//, you may
add new //modules// to the language, which adds functionality to the
language, like [link label="links"](https://modmark.org), lists and more!
```

The `[link]` syntax invokes a **module**, which lives outside the language itself, and the ability to use custom modules enhances the language and gives it capabilities way beyond what one single syntax set may. Make sure to check out the [playground](https://modmark.org) to see an example document which uses modules!

You can see more of the syntax [here](MODMARK.md), or continue reading to find out more about the project structure and technical details.

## Structure

The project is subdivided into multiple different parts:

- [core](core) - the core of the language, that is, the code which actually transforms a ModMark document to another format, keeps track of the loaded packages and the modules they contain, and delegates the transformation to different packages and modules. It also contains the source code for some native modules, which are special modules that are built-in into the language itself.
- [parser](parser) - the parser that parses the document itself into different syntactical expressions. You can see the raw output of the parser in the [playground](https://modmark.org) by changing the view to `Abstract syntax tree`.
- [cli](cli) - the cli tool, giving you the ability to compile ModMark documents locally on your computer.
- [playground](playground) - the code for the [online playground](https://modmark.org) and a build script that compiles the project to [webassembly](https://webassembly.org) to be able to run it online.
- [packages](packages) - our own developed packages, and instructions on how to write your own packages. Most importantly, it contains the language packages [html](packages/html) and [latex](packages/latex) which allows [core](core) to output HTML and LaTeX documents. Note that these packages are not privileged in any way - they are compiled in the same way and uses the same interfaces that custom third-party packages may - but they may be bundled in the binary of some of our first-party distributions.
- [package-tests](package-tests) - code to test our packages. Since the [packages](packages) are completely standalone, they are not part of the larger cargo workspace and thus needs to be tested by a custom test script rather than by normal cargo tests.

Each of these parts lives in their own directory, and contains their own readme file. Go to one of these directories for more specific information.

## Technical details

The language itself is written in [Rust](https://rust-lang.org), and the packages it may load are [WASI binaries](https://wasi.dev), which is [WebAssembly](https://webassembly.org) binaries with a system interface allowing them to act like terminal tools, being able to read `stdin`, `stderr`, `stdout`, environment variables, program arguments and so on. The [core](core) uses these system interfaces to interface with the packages themselves. The data to transform is sent via `stdin`, like piping the input data to a terminal program, and the resulting data is expected to be sent to `stdout`.

You may write your own packages in any language that may be compiled to [WASI](https://wasi.dev), and it isn't much harder than reading the program arguments and `stdin`, and printing the result to `stdout`. More information can be found in [packages/readme](packages/README.md).

The [WASI](https://wasi.dev) binaries are being loaded into the WASM runtime [Wasmer](https://wasmer.io), and runs in a sandboxed environment, giving the appropriate input to `stdin` and capturing `stdout`/`stderr`. Targeting the web (as in the [playground](playground)), web bindings are generated for interfacing with the [core](core), and a HTML/CSS/JS site interfaces with these bindings. The project is thus compiled to wasm, and [Wasmer](https://wasmer.io) provides a sandbox for us to control the input to the [WASI](https://wasi.dev) packages.

For bundling the standard packages, if the feature `core/bundle_std_packages` is enabled (which it is by default), all packages are compiled targeting `wasm32-wasi` which yields one `.wasm` file for each package. These packages are then copied into the binary, and will then be loaded by [core](core) the first time a document is compiled.

The language itself is completely output-agnostic, meaning that it may support any output format and the syntax doesn't change. Packages, like [`math`](packages/math), declares what output formats their declared modules support (in this case HTML and LaTeX), and will get the desired output format as an argument when called. When the document gets compiled and finds a `[math]` module, [core](core) will check if there is a transform from that module to the wanted output format. This means that even if the package implementing the default `[math]` module doesn't support it, one could just make a new package declaring a transform to the output format of their liking. In addition to this, there are so-called _language packages_ which are packages responsible for transforming tags and other structural components, such as the document itself, to one output format. Examples of these are the [html](packages/html) and [latex](packages/latex) packages, but one could also write a language package of their own, or import one made by another third-party. There is nothing that sets a _language package_ apart from any other package, but the ability to transform the whole document to the final output format as well as transforming tags and such to the final output format. It would thus probably be easy to make a new package supporting output formats like RTF or Markdown.

## License

This project is licensed under [Apache 2.0](LICENSE) license, © 2023 The ModMark Contributors. Third party software is used, which are licensed under their respective licenses, which can be found under [licenses.txt](licenses.txt).
