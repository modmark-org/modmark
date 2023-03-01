# ModMark Syntax Description

*Examples are written as they would look in source text, before compilation. Note that wrapping may cause the text to start new rows in unintended locations.*

## Introduction

ModMark is a lightweight markup language that is centered around modules. These modules are either part of the language or user created, and they are used to customize output.

Tags and smart punctuation are also part of the language. Information about these can be found in the reference document.

## Paragraphs

Each paragraph consist of one or more lines of text, and ends by one or many blank rows, similar to markdown. Line breaks will be kept and may or may not be rendered by the output format. Many output formats such as HTML, Markdown and LaTeX doesn't render line breaks. If you want to escape the line break to ensure that it is outputted on the same line, you can put a backslash at the end of the line. Here is an example of some paragraphs:

**Example: Paragraphs**

    This is the first sentence of the first paragraph.
    This line starts on another row. These rows belong to the 
    same paragraph and in many output formats they will appear
    on the same row.

    This is the first sentence of the second paragraph. \
    Each empty row marks the end of a paragraph. The backslashes \
    at the end of all these rows ensure that they are all parsed \
    as being part of the same row.


    Multiple empty rows are treated as one, which means this is the third paragraph.

## Modules

Modules are programs that are run from within your document, transforming the text you give them at their discretion. To use, or *invoke*, a module, encase its name within square brackets, and place the text you want to pass to it afterwards: `[link] https://example.com`. The module, in this case `link`, is invoked, consuming the text you give it, `https://example.com`.

There are two ways to invoke a module, **inline** or **multiline**. When invoking a module in the middle of a paragraph, you do it **inline**. The module will consume the text following it, until the next space or until the end of the line, so you can write `Then, you take [math] x^2 and find out ...`. If you invoke the module **multiline**, you write it as if it was its own paragraph, with one empty row before and after it. It may then consume multiple rows. Here is one example of that:

**Example: Multiline Module**
```text
... will be attached below:

[code]
def foo():
    print("Hello world!")

This code defines ...
```

Different modules may have widely different behaviour, but the intention is for modules invoked **inline** to insert content in-place, inserted into the body of the paragraph, while **multiline** modules may generate content that takes up large chunks of the document. The name of a module may consist of letters, digits, hyphens and underscores.

This works fine most of the time, but sometimes you may want to include multiple words to a module you invoke. This is the place where *delimiters* come in. Immediately following the square brackets, you may define your own *opening delimiter*. If you do, all text following up until the matching *closing delimiter* is taken, like this: `... a more complex equation like [math](x^2 + y^2 + z^2) may be ...`. In this case, the module gets passed all the text `x^2 + y^2 + z^2`, including the spaces but not including the brackets. You may use any non-alphanumeric character as the opening delimiter, and if you use brackets (like in the example above), the matching closing delimiter has the opposing bracket used. For **inline** modules, one character is allowed as the delimiter, and for **multiline** modules, any amount of characters is allowed. For a multiline module, the opening delimiter `{{(` is matched by the closing delimiter `)}}`. Here is some more examples of delimiter usage:

**Example: Modules with delimiters**

    [mymodule]{&^
    This text is included in mymodule, as expected.

    But also this text because it is inside the delimiters.
    ^&}

    Inline modules can also use delimiters to capture [mymodule]{ all this text }
    
    However [mymodule]{ even if delimiters are used, 
    this text will not be included since inline modules stops
    at the end of the row }. Since the module isn't closed, it
    will not be parsed as a module but rather as text.

    Here, [mymodule](( the module body will include the inner layer of parentheses, since only one character is allowed as the delimiter ))

If you insert a large chunk of C code, an opening delimiter of `{` will likely find a matching `}` within the code block, while an opening delimiter of `<` may work in this case, but not if inserting HTML. By being able to choose your own custom delimiter, you will always be able to use one which isn't included in your text (you can go crazy, like `<(({{{&%%%((`)

Some modules may accept arguments to configure their behaviour in one specific instance. These arguments can either be positional, or explicitly named, and are placed within the square brackets. If the `url` module takes the argument `color`, you may write `[url color=pink] https://example.com`, or `[url pink] https://example.com`. Writing the name explicitly makes it clear what the argument refers to, while not writing it explicitly makes it take up less space. Each module declares what arguments it takes, and if they are optional or mandatory, so see the documentation for the modules you use! Writing `[module a b c]` provides three arguments, `a`, `b` and `c` to the module; if you only intended to provide one, use quotes, `[module "a b c"]`. Arguments are separated by spaces, and quotes are required if passing values containing other characters than letters, digits and underscores. If you want to mix positional and named arguments, that is fine as long as you use the positional arguments first. For multiline modules, you may place arguments on different lines. Here are some examples of module usage with arguments:

**Example: Module with arguments**

    [mymodule red apple indent=4]
    This module is provided with the positional arguments "red" and "apple" and the named argument "indent=4"

    [code
        lang=python
        indent=tabs
        tab_size=4]
    def foo():
        print("Hello world!")

    The equation [math style=italic] x^2 is simple enough, but for [math style=bold]!x^2 + y^3!, you'll need to study multivariable calculus

## Escaping characters

You may escape the upcoming character by using a backslash. The escaped character will be rendered as-is, and may not be treated as a part of a module, tag, escape character or smart punctuation. The only exception to this is a backslash just before a line break. In that case, the line break gets removed.

## Tags

Similarly to Markdown, ModMark supports tags. Tags are character sequences that may encase text and/or inline modules, and is used to apply additional formatting. An example of one such tag is `**`, which makes the encased text bold: `**abc**` will be rendered as **abc**. All opening and closing tags in the language by default consists of one symbol repeated two times. Note that more tags may be added in upcoming releases, and upcoming releases may also support customized tags. Some tags allow nesting: `**bold //italic and bold//**` will be rendered as **bold *italic and bold***, while some tags, like <code>\``verbatim``</code>, doesn't allow for nested tags. Here are the tags supported by default:

| Source                     | Output                         | Allows nested tags |
|----------------------------|--------------------------------|--------------------|
| ```**bold**```             | <strong>bold</strong>          | yes                |
| ```//italic//```           | <em>italic</em>                | yes                |
| ```__subscript__```        | <sub>subscript</sub>           | yes                |
| ```^^superscript^^```      | <sup>superscript</sup>         | yes                |
| <code>\``verbatim``</code> | verbatim                       | no                 |
| ```==underlined==```       | <ins>underlined</ins>          | yes                |
| ```~~strikethrough~~```    | <strike>strikethrough</strike> | yes                |
| ```$$math x^2$$```         | <pre>math x<sup>2<sup></pre>   | no                 |

Tags will always match the first possible closing tag: `**bold***` will have the last star outside the bold tag. As described above, a backslash escapes *one character*, so `**bold\***` will make the first star in the second cluster be just the `*` character, and thus be put inside the bold tag.

## Headings

Headings may occur as the first line of a paragraph, or by themselves, and start with a number of hashtags `#`. One hashtag corresponds to a heading at level 1, two corresponds to a heading at level 2 etc. Any space before the content of the heading will be stripped. Note that while ModMark supports any heading level, not all output formats does. For example, HTML only has headings up to level 6, and LaTeX only supports headings up to level 3 (subsubsection).

| Source                    | Output                   |
|---------------------------|--------------------------|
| ```# heading level 1```   | <h1>heading level 1</h1> |
| ```## heading level 2```  | <h2>heading level 2</h2> |
| ```### heading level 3``` | <h3>heading level 3</h3> |

## Smart punctuation

Smart punctuation is character sequences that gets replaced by a different character, or encasement in quotes where the quotes gets switched out for left and right quotes. `...` will get replaced by the ellipsis character &hellip;, and "abc" will be replaced with &ldquo;abc&ldquo;. Escaping also applies to these, `\...` will not become an ellipsis and `\....` will become a dot followed by an ellipsis.

| Source                | Output                      |
|-----------------------|-----------------------------|
| ```"double quotes"``` | &ldquo;double quotes&ldquo; |
| ```'single quotes'``` | &lsquo;single quotes&rsquo; |
| ```a -- b```          | a &ndash; b                 |
| ```a --- b```         | a &mdash; b                 |
| ```ellipsis...```     | ellipsis&hellip;            |

More dashes or dots than the amount corresponding to any smart punctuation sequence will not be replaced. Thus, `....` corresponds to four dots, and `-----` to five dashes.
