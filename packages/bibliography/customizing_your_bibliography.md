# Bibliography package
This is the Bibliography package. It provides support for inline citations with `[cite]`
and managing a bibliography database and printing a bibliography with `[bibliography]`.
The included `[bibliography]` module uses [typst/hayagriva](https://github.com/typst/hayagriva)
as the backend and supports some styles included within.

## How it all works
The `[cite]` module is 'dumb' and doesn't control how it is displayed at all. It contains
very little internal logic, and doesn't have access to the bibliography database. More or less,
it just pushes its argument and body to the list `inline_citations` which, after evaluating
all `[cite]`s, contains a list of all citations in the document and their content.

Moreover, `[cite]` generates a module which later on reads the variable `inline_citation_labels`,
which for each citation contains the JSON data the `[cite]` should be replaced with. This means
that a `[cite]` acknowledges its existence by pushing itself to `inline_citations`, and then
reads `inline_citation_labels` and represents itself by whatever content it finds in
`inline_citation_labels` for that specific label.

The `[bibliography]` module is responsible not only for reading and printing the bibliography,
but also resolving citations. It reads `inline_citations` and, for each citation, pushes one
label for each (unique) citation. At the time of evaluating `[bibliography]`, it thus has
access to the bibliography database and all citations and may format all citations and the
bibliography at its discretion.

## Configuring `[bibliography]`
`[bibliography]` has four arguments:
* `style`, which can be set to `IEEE`/`APA`/`MLA`/`Chicago` to configure the style for inline citations and the bibliography.
* `file`, which can be set to a file to read a BibLaTeX or Hayagriva YAML database from. If this is empty, the body of the module is used as the database.
* `visibility`, which can be set to `visible`/`hidden` to show or hide the bibliography. Note that since `[bibliography]` is the module resolving inline citations, a `[bibliography]` must exist - this argument allows the user to hide the bibliography while still using inline citations.
* `unused-entries`, which can be set to `visible`/`hidden` to show or hide entries which aren't cited in the document.

The bibliography and citations themselves are mainly just text-based, so these modules may be
used regardless of output format. Some bibliography styles needs styling and links, so 
`__italic`, `__bold` and `[link]` (in addition to `__text`) is needed for the output format.
Additionally, for HTML and LaTeX, hyperlinks from inline citations to the bibliography entries
are inserted.

## Making your own `[bibliography]`
The built-in bibliography module may not be a perfect fit for all use cases. It may be replaced
like any other module by doing `import std:bibliography hiding bibliography` and importing your
own. Since the `[cite]` modules doesn't contain much logic but leverages the scheduler in a
clever way, they may be re-used in a custom `[bibliography]` implementation.

To leverage the `[cite]` system, the `[bibliography]` should declare these variable accesses:
* `"inline_citations": {"type": "list", "access": "read"}`
* `"inline_citation_labels": {"type": "set", "access": "add"}`

Then, the module should read the `inline_citations` list to find out what citations are used in
the text. Each entry is a JSON object containing the keys `key`, which is the database key cited
and optionally a `note` which is an additional note. `[cite "p. 10"] foo` would be represented
by `{"key": "foo", "note": "p. 10"}`. The module should generate a citation for each entry in the
`inline_citations` list, generate output as a JSON array valid as a module output, and then push
the original JSON concatenated with the output JSON to the set `inline_citation_labels`. It is of
highest importance that the original JSON **is identical to the one in the original list**,
otherwise it may not be picked up. The citation `{"key":"modmark"}` may be resolved by pushing
`{"key":"modmark"}[{"name":"__text","data":"[1]"}]` to `inline_citation_labels`. Since
`[bibliography]` is free to push any JSON to `inline_citation_labels`, it is in full control of
how every citation is rendered in the output format.
