# ModMark: Parser
This is the parser for the ModMark language

## Testing
Integration tests that aim to test a successful compilation of the document and want to assert a certain tree structure should be placed in `tests/compilation_tests`. Such tests come in two flavors: unified and split. Both of them take a ModMark text as input, parses it, and compares the resulting tree to an expected JSON structure. If there is a mismatch, the test fails and both the expected and actual structure is printed. The ModMark file is compiled two times, once using LF line endings and once using CRLF line endings.

 * Split tests have two files, `test_name.mdm` and `test_name.json` containing the input ModMark and expected JSON structure respectively.
 * Unified test have one file, `test_name.mdmtest`. The file is formatted in Markdown and have two code blocks, the first one with the input, marked `modmark` or `mdm`, and the second one with the output marked `json`. Lines starting with triple backticks may not occur in the input or output. Arbitrary text, like comments, may be placed before, after and in between the two code blocks. It might be easier to see both the input and output in the same file and some IDE:s may allow for syntax highlighting inside the code blocks.

Two different tests may not have the same name.
