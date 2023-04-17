# ModMark CLI Interface

ModMark can be run locally to compile documents using this CLI interface.

## How to run

### Compile

To compile a file in modmark, the CLI can be used like this:

```
$ modmark compile [OPTIONS] <INPUT> <OUTPUT>
```

The `<INPUT>` should be a path to the input document, and the `<OUTPUT>` should be a path to the output document. You may run it like this:

```
$ modmark in.mdm out.html
```

This would compile the file `in.mdm` and output the compiled HTML file as `out.html`. The format to compile to is inferred by the output file extension, if you use `out.html` as the output file, it will compile the file to HTML and if you use `out.tex`, it will compile the file to LaTeX.

**Optional flags**

| Flag             | Usage                                                                    |
| ---------------- | ------------------------------------------------------------------------ |
| `-f`/`--format`  | `-f <FORMAT>` overrides the inferred output format with the one supplied |
| `-w`/`--watch`   | Watches the input file for changes and re-compiles at every change       |
| `-d`/`--dev`     | Prints the parsed AST tree before compiling                              |
| `-V`/`--version` | Prints the version of the CLI took                                       |
| `-h`/`--help`    | Prints the usage information                                             |

### Cache

To handle the cache of packages the CLI can be used like this:

Uninstalls all the cached packages

```
$ modmark cache clear
```

Lists all the cached packages:

```
$ modmark cache list
```

Prints the location of the cached packages:

```
$ modmark cache location
```

## Compilation

You may build the binary using `cargo b -p modmark`.
The CLI uses [`core`](../core) to compile the document, which in turn may compile and bundle the built-in standard packages. Compiling these packages require the `wasm32-wasi` target which can easily be installed by rustup; `rustup target add wasm32-wasi`.
Bundling the built-in standard packages is controlled by the feature `core/bundle_std_packages` which is enabled by default, so if you want to build without bundling the standard packages, use the `--no-deafult-features` flag when building.
