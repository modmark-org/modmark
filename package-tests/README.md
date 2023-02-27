# package-tests

This crate tests the packages found in the `./packages` directory. Each package in there is its own crate, but they are not part of the larger workspace. This means that commands such as `cargo build` doesn't build them directly (rather, `core` builds them). Since we want a custom build flow for those packages, this is how we have to do it, but it is problematic with tests. Testing a packages isn't as simple as `cargo test`, in that case we would have to go into every package directory and run `cargo test` within each of them. This isn't really optimal.

This crate is aimed at solving that shortcoming. The packages themselves are focused on being small to compile to `wasi`, and all interfacing with them occurs with `stdin`/`stdout`. This means that we can do simple integration testing of the package itself by providing an input and output file. During development, creating a sample json file is useful since it can be piped into the program to see if it does what it expects, and this crate gives a simple path from that to a full-fledged test case.

## Convert `example.json` to a test case

Let's say you are developing the `html` module, and you have an input file `example.json` with this content:

```json
{
    "name": "__bold",
    "arguments": {},
    "children": [
        {
            "name": "__text",
            "data": "Hello, world",
            "arguments": {},
            "inline": true
        }
    ]
}
```

The file is simple enough and useful to find bugs in the code during development. You can run your application using `cat example.json | cargo run -- transform __bold html` When development is finished, it would be nice to have this as a test case, and the only things you have to do to convert this to an actual test case which is checked by `cargo test` is the following:

1. Move the file to the `tests` directory.
    * If you don't have a `tests` directory, make one as a sibling to `src`.
    * Optionally, give the file a more descriptive name of what is being tested
2. Add a key `__test_transform_to` with the value of what you use as the third argument to your package, which in this case is `html`. The first argument is retrieved from the node name.
3. Add a key `__test_expected_result` with the value of the resulting json you expect.

That's it! This crate will find all `tests/*.json` files one level deep within the packages folder and run them all on `cargo test`. Feel free to add multiple tests confirming that everything works as expected.
