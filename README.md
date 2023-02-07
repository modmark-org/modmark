# modmark

## How to run

The current user interface is a cli-tool that can be run using the following:

```
$ modmark [OPTIONS] <INPUT-PATH> <OUTPUT-PATH>
```

An example of this would be:

```
$ modmark x.txt output/y.html
```

This would compile the file `x.txt` and output into the target file `y.html` in the folder `output`. Note how the user decides the file extension.

**Optional flags**

- `-d`, `--dev` &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; Prints the ATS-tree every compilation.
- `-w`, `--watch` &nbsp;&nbsp;&nbsp; Watches the file and recompiles on change.
- `-h`, `--help` &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;For more information
