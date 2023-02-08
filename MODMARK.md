
# ModMark: Description

*Examples are written as they would look in source text, before compilation.*

## Introduction

ModMark is a lightweight markup language that is centered around modules. These modules are either part of the language or user created, and they are used to customize output.

Tags and smart punctuation are also part of the language. Information about these can be found in the reference document.

## Paragraphs

Paragraphs are sequences of text separated by one or more empty rows. A single backslash is used at the end of a row to join the following row in the compiled text. Double backslash is used to include a break in the compiled text without creating a new paragraph.

Example 1: simple paragraphs

    This is the first sentence of the first paragraph.
    This starts on another row. These rows belong to the same paragraph and in many output formats they will appear on the same row.

    This is the first sentence of the second paragraph. Each empty row marks the end of a paragraph.


    Multiple empty rows are treated as one, which means this is the third paragraph.

Example 2: paragraphs with escapes

    A single backslash can be used like this \
    to ensure that these two rows will be joined.

    A double backslash can be used like this \\
    to force a break between these two rows, without creating a new paragraph.

    You can also use double backslashes like this \\
    \\
    which creates an empty row in the same paragraph.

## Modules

Modules are invoked by an opening and closing square bracket. Information about the module is kept inside the square brackets, while the contents of the module is what follows. Exactly how much is consumed by the module depends on the closing delimiter, and whether the module is located inline. A module is automatically treated as inline if there is text outside the square brackets on the same row, otherwise it is treated as multiline.

By default, an inline module will consume the next "chunk" of text, meaning the sequence of characters until a whitespace or newline character is found. This does not include space immediately following the closing square bracket. A multiline module on defaults to consuming until an empty row is found.

Delimiters are patterns specified after the closing square bracket. A module will then attempt to consume until the matching closing delimiter is found. However, inline modules are still limited to the row they were defined on, which means that they will close at the end of the row if no matching delimiter is found.

Example 3: basic module usage

    This is a paragraph with some text in it.

    [mymodule]
    This is a multiline module invocation without a closing delimiter.
    Anything that is written until an empty row is found will be included in mymodule.

    Another paragraph starts here, so [mymodule]consumesthistext because it is inline.

    An inline module also [mymodule] consumesthistext even with space between.

Closing delimiter patterns will mirror the opening pattern and individual bracket characters. The pattern "{{(" will be closed by ")}}". In inline modules, only the first characters of the following text will be recognized as the delimiter.

Example 4: modules with delimiters

    [mymodule]{&^
    This text is included in mymodule, as expected.

    But also this text because it is inside the delimiters.
    ^&}

    Inline modules [mymodule]{ can also use delimiters to capture all this text }
    
    However [mymodule]{ even if delimiters are used, this text will not be included since inline stops at the end of the row }

Modules can also take arguments. These arguments can either be positional, or explicitly named. Positional arguments must appear before named arguments. Arguments are separated by whitespace or newlines.

Example 5: module with arguments

    [mymodule red apple indent=4]
    This module is provided with the positional arguments "red" and "apple" and the named argument "indent=4"

Example 6: inline module with double parentheses

    Here, [mymodule](( will include the inner layer of parentheses ))

## Limitations and valid characters

Type | Valid characters
---          | ---
text         | alphanumeric, special characters if <br> escaped or no open-close match
module names | alphanumeric, underscores, hyphens
module args  | alphanumeric, underscores
delimiters   | all special characters

There are also limitations to smart punctuation. Character sequences that could technically be broken up into smaller sequences for smart punctuation will instead be used as is. This means that "----" is not parsed as "&ndash;&ndash;". 
