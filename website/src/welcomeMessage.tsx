const welcomeMessage = `# ModMark
Hello and welcome to ModMark!
ModMark is a lightweight markup language focused on modularity.
It allows for syntactical constructions similar to Markdown,
such as **bold text**, //italic text//, ~~some ==crazy
__combination__==~~, but the coolest feature of all are modules:

Here, we invoke the table module, and print [math][x], [math][x^2]
and [math][x^3] for [math][x] between [math][1] and [math][4]. The
input to the table module contains some math modules, and that
works just fine!

[table]
[math][x]|[math][x^2]|[math][x^3]
1|1|1
2|4|8
3|9|27
4|16|64

What is this //table// thing? It is a module. Modules live
within packages, which are programs who reside outside language.
They are simply .wasm-programs which gets the input of the
module, in this case all the text in the paragraph starting
with \[table], and can do anything it want with it. In
this case, ModMark sends the text together with information such
as that the target output format is HTML, to the package which
houses the \[table] module. The module then generates
the corresponding <table> and </table> tags, and the result
appears here.

So, modular you say? How modular? Well, a package is responsible
for translating something to something else, like the table module
to HTML, but it may also be responsible for translating, let's say,
any bold text to HTML. **This is some //italic// text within some
bold** text, and it is all transformed to HTML by a package, simply
called **HTML**. That's right. The output language is itself
implemented in a package. You can thus write a package yourself
which takes a ModMark document and turns it into, let's say,
Markdown, LaTeX, or why not RTF, or anything of your liking.

At the top of this "ModMark Playground", there is a button to view
all loaded packages, and see the transforms and modules they define.
`;
export default welcomeMessage;