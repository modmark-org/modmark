
# ModMark: Description

*Examples are written as they would look in source text, before compilation. Note that wrapping may cause the text to start new rows in unintended locations.*

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

Modules are programs that are run from within your document, transforming the text you give them at their discretion. To use, or *invoke*, a module, encase its name within square brackets, and place the text you want to pass to it afterwards: `[url] https://example.com`. The module, in this case `url`, is invoked, consuming the text you give it, `https://example.com`.

There are two ways to invoke a module, **inline** or **multiline**. When invoking a module in the middle of a paragraph, you do it **inline**. The module will consume the text following it, until the next space or until the end of the line, so you can write `Then, you take [math] x^2 and find out ...`. If you invoke the module **multiline**, you write it as if it was its own paragraph, with one empty row before and after it. It may then consume multiple rows. Here is one example of that:

```text
... will be attached below:

[code]
def foo():
    print("Hello world!")

This code defines ...
```

Different modules may have widely different behaviour, but the intention is for modules invoked **inline** to insert content in-place, inserted into the body of the paragraph, while **multiline** modules may generate content that takes up large chunks of the document. The name of a module may consist of letters, digits, hyphens and underscores.

This works fine most of the time, but sometimes you may want to include multiple words to a module you invoke. This is the place where *delimiters* come in. Immediately following the square brackets, you may define your own *opening delimiter*. If you do, all text following up until the matching *closing delimiter* is taken, like this: `... a more complex equation like [math](x^2 + y^2 + z^2) may be ...`. In this case, the module gets passed all the text `x^2 + y^2 + z^2`, including the spaces but not including the brackets. You may use any non-alphanumeric character as the opening delimiter, and if you use brackets (like in the example above), the matching closing delimiter has the opposing bracket used. For **inline** modules, one character is allowed as the delimiter, and for **multiline** modules, any amount of characters is allowed. For a multiline module, the opening delimiter `{{(` is matched by the closing delimiter `)}}`. Here is some more examples of delimiter usage:

Example 4: modules with delimiters

    [mymodule]{&^
    This text is included in mymodule, as expected.

    But also this text because it is inside the delimiters.
    ^&}

    Inline modules can also use delimiters to capture [mymodule]{ all this text }
    
    However [mymodule]{ even if delimiters are used, 
    this text will not be included since inline stops at the end of the row }

    Here, [mymodule](( the module body will include the inner layer of parentheses, since only one character is allowed as the delimiter ))

If you insert a large chunk of C code, an opening delimiter of `{` will likely find a matching `}` within the code block, while an opening delimiter of `<` may work in this case, but not if inserting HTML. By being able to choose your own custom delimiter, you will always be able to use one which isn't included in your text (you can go crazy, like `<(({{{&%%%((`)

Some modules may accept arguments to configure their behaviour in one specific instance. These arguments can either be positional, or explicitly named, and are placed within the square brackets. If the `url` module takes the argument `color`, you may write `[url color=pink] https://example.com`, or `[url pink] https://example.com`. Writing the name explicitly makes it clear what the argument refers to, while not writing it explicitly makes it take up less space. Each module declares what arguments it takes, and if they are optional or mandatory, so see the documentation for the modules you use! Writing `[module a b c]` provides three arguments, `a`, `b` and `c` to the module; if you only intended to provide one, use quotes, `[module "a b c"]`. Arguments are separated by spaces, and quotes are required if passing values containing other characters than letters, digits and underscores. If you want to mix positional and named arguments, that is fine as long as you use the positional arguments first. For multiline modules, you may place arguments on different lines. Here are some examples of module usage with arguments:

Example 5: module with arguments

    [mymodule red apple indent=4]
    This module is provided with the positional arguments "red" and "apple" and the named argument "indent=4"

    [code
        lang=python
        indent=tabs
        tab_size=4]
    def foo():
        print("Hello world!")

    The equation [math style=italic] x^2 is simple enough, but for [math style=bold]!x^2 + y^3!, you'll need to study multivariable calculus

## Limitations and valid characters

There are limitations to smart punctuation. Character sequences that could technically be broken up into smaller sequences for smart punctuation will instead be used as is. This means that "----" is not parsed as "&ndash;&ndash;".
