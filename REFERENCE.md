# ModMark: Syntax reference

## Default tags

Source                          | Output                         | Allows nested tags
---                             | ---                            | ---
```**bold**```                  | <strong>bold</strong>          | yes
```//italic//```                | <em>italic</em>                | yes 
```__subscript__```             | <sub>subscript</sub>           | yes
```^^superscript^^```           | <sup>superscript</sup>         | yes
<code>\``verbatim``</code>      | verbatim                       | no
```==underlined==```            | <ins>underlined</ins>          | yes
```~~strikethrough~~```         | <strike>strikethrough</strike> | yes
```$$math x^2$$```              | <pre>math x<sup>2<sup></pre>   | no

## Headings

Source                      | Output
---                         | ---
```# heading level 1```     | <h1>heading level 1</h1>
```## heading level 2```    | <h2>heading level 2</h2>
```### heading level 3```   | <h3>heading level 3</h3>

## Smart punctuation

Source                  | Output
---                     | ---
```"double quotes"```   | &ldquo;double quotes&ldquo;
```'single quotes'```   | &lsquo;single quotes&rsquo;
```a -- b```            | a &ndash; b
```a --- b```           | a &mdash; b
```ellipsis...```       | ellipsis&hellip;