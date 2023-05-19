import styled from "styled-components";

const Container = styled.div`
    display: flex;
    flex-direction: column;
    padding: 1rem;
    max-width: 700px;
`
const SyntaxTable = styled.div`
    display: grid;
    grid-template-columns: 12rem 1fr;
    grid-row-gap: 0.5rem;
    max-width: 420px;
`

const Border = styled.div`
    border-bottom: 1px solid #aaa;
    grid-column: 1 / span 2;
`

const Example = styled.p`
    font-family: "JetBrains Mono", monospace;
    background: #f1f1f1;
    padding: 1rem;
    border-radius: 0.5rem;
    width: fit-content;
`

const Code = styled.code`
    font-family: "JetBrains Mono", monospace;
    background: #f1f1f1;
`

export default function Guide() {
    return <Container>
        <h1>Guide</h1>
        <p>
            ModMark is a lightweight markup language that is centered around modules.
            These modules are either part of the language or user created, and they are used to customize output.
            Tags and smart punctuation are also part of the language.
        </p>
        <h2>Basic syntax</h2>
        <h3>Paragraphs</h3>
        <p>
            Each paragraph consist of one or more lines of text, and ends by one
            or many blank rows, similar to markdown. Line breaks will be kept and
            may or may not be rendered by the output format. Many output formats
            such as HTML, Markdown and LaTeX doesn't render line breaks. If you
            want to escape the line break to ensure that it is outputted on the
            same line, you can put a backslash at the end of the line. Here is
            an example of some paragraphs:
        </p>
        <Example>
            This is the first sentence of the first paragraph. <br/>
            This line starts on another row. These rows belong to the <br/>
            same paragraph and in many output formats they will appear <br/>
            on the same row. <br/>
            <br/>
            This is the first sentence of the second paragraph. \ <br/>
            Each empty row marks the end of a paragraph. The backslashes \ <br/>
            at the end of all these rows ensure that they are all parsed \ <br/>
            as being part of the same row. \ <br/>
            <br/>
            Multiple empty rows are treated as one, which means this is the third paragraph.
        </Example>
        <p>
            In the example above, the backslashes are used to escape the line breaks.
            However, <strong>backslashes can be used to escape any character</strong>, including other backslashes.
            This will render the escaped character as-is.
        </p>
        <h3>Built-in tags</h3>
        <p>Here is a table for the basic text formatting, or tags, built into ModMark</p>
        <SyntaxTable>
            <div>Name</div>
            <div>Example</div>
            <Border/>
            <div>Bold</div>
            <div><strong>**bold**</strong></div>
            <div>Italic</div>
            <div><em>//italic//</em></div>
            <div>Heading 1</div>
            <div># Level 1</div>
            <div>Heading 2</div>
            <div>## Level 2 and so on...</div>
            <div>Subscript</div>
            <div><sub>__sub__</sub>script</div>
            <div>Superscript</div>
            <div><sup>^^super^^</sup>script</div>
            <div>Verbatim</div>
            <div><code>``verbatim``</code></div>
            <div>Underline</div>
            <div><u>==underline==</u></div>
            <div>Strikethrough</div>
            <div>
                <del>~~strikethrough~~</del>
            </div>
            <div>Math</div>
            <div><code><a href="https://www.overleaf.com/learn/latex/Mathematical_expressions">$$latex math syntax$$</a></code>
            </div>
        </SyntaxTable>
        <h3>Smart punctuation</h3>
        <p>Smart punctuation is character sequences that gets replaced by a different
            character, or encasement in quotes where the quotes gets switched out
            for left and right quotes. ... will get replaced by the ellipsis
            character …, and "abc" will be replaced with “abc“. Escaping also applies
            to these, \... will not become an ellipsis and \.... will become a
            dot followed by an ellipsis.
        </p>
        <h2>Modules</h2>
        <p>
            The main feature of ModMark is the module and package system. Modules are programs
            that are run from within your document, transforming the text you
            give them at their discretion. A package is a collection of modules.
            Many packages are included as a part of the standard library, the
            easiest way to view these is by using the <em>package docs</em> button
            in the playground. Any packages that are imported will also
            be available in the package docs.
        </p>
        <h3>Importing</h3>
        <p>
            Imports are written att the top of the document, as a part of a
            special [config] module. For example:
        </p>
        <Example>
            [config] <br/>
            import myPackage.wasm <br/>
            import catalog:robber <br/>
            import https://example.com/myPackage.wasm <br/>
            import std:link hiding reference <br/>
        </Example>
        <p>
            The first import is a local import, it imports the file myPackage.wasm.
            The second import is an import from the <a href="https://github.com/modmark-org/package-registry">package
            registry</a>.
            The third import is an import from a remote URL that points to a wasm file.
            The fourth import is an import from the standard library, and it
            reimports the link module but hides the reference module.
        </p>
        <h3>Using a module</h3>
        <p>
            There are two ways to invoke a module, inline or multiline. When
            invoking a module in the middle of a paragraph, you do it inline.
            The module will consume the text following it, until the next space
            or until the end of the line, so you can
            write <Code>Then, you take [math] x^2 and find out ...</Code>.
            If you invoke
            the module multiline, you write it as if it was its own paragraph,
            with one empty row before and after it. It may then consume multiple
            rows. Here is one example of that:
        </p>
        <Example>
            ... will be attached below: <br/>
            <br/>
            [code] <br/>
            def foo(): <br/>
            &emsp; print("Hello world!") <br/>
            <br/>
            This code defines ... <br/>
        </Example>
        <p>
            Different modules may have widely different behaviour, but the intention
            is for modules invoked inline to insert content in-place, inserted into
            the body of the paragraph, while multiline modules may generate content
            that takes up large chunks of the document. The name of a module may
            consist of letters, digits, hyphens and underscores. Worth noting is
            that escaping modules work the same way as escaping anything else,
            so if you want to write <Code>[module]</Code> as-is without using the
            module, you can write <Code>\[module]</Code>.
        </p>
        <p>
            If you want to have blank lines in a multiline module, or multiple words
            in an inline module, <em>delimiters</em> can be used.
        </p>
        <Example>
            [mymodule]{'{'}&^ <br/>
            This text is included in mymodule, as expected. <br/>
            <br/>
            But also this text because it is inside the delimiters. <br/>
            ^&{'}'} <br/>
            <br/>
            Inline modules can also use delimiters to capture [mymodule]{'{'}all this text {'}'} <br/>
            <br/>
            However [mymodule]{'{'}even if delimiters are used, <br/>
            this text will not be included since inline modules stops <br/>
            at the end of the row {'}'}. Since the module isn't closed, it <br/>
            will not be parsed as a module but rather as text. <br/>
            <br/>
            Here, [mymodule](( the module body will include the inner layer of <br/>
            parentheses, since only one character is allowed as the delimiter for inline modules. ))
        </Example>
        <p>
            If you insert a large chunk of C code, an opening delimiter
            of <Code>{'{'}</Code> will likely find a matching <Code>{'}'}</Code> within
            the code block, while an opening delimiter of <Code>{'<'}</Code> may
            work in this case, but not if inserting HTML. By being able to
            choose your own custom delimiter, you will always be able to use one
            which isn't included in your text (you can go crazy,
            like <Code>{'<(({{{&%%%(('}</Code>).
        </p>

    </Container>
}
