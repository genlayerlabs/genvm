Specification resides in `doc/website/src/spec` in rst (reStructuredText) format

Info about rst format:
- links must follow https://www.sphinx-doc.org/en/master/usage/referencing.html#cross-referencing-documents
> :doc:
    Link to the specified document; the document name can be a relative or absolute path and is always case-sensitive, even on Windows. For example, if the reference :doc:`parrot` occurs in the document sketches/index, then the link refers to sketches/parrot. If the reference is :doc:`/people` or :doc:`../people`, the link refers to people.

Specification requirements:
- no information duplication
- when mentioning another specification document put a link to it
- no making up facts: all must be gathered from code or asked from a user
- all files must be listed in appropriate `index.rst`
- do not include implementation file names
- "future improvements" and similar section are forbidden. Only current content must be present
- specification is not a place for "best practices"
- each directory must contain an `index.rst` with a toc tree
- all terms must be linked to glossary

Glossary requirements:
- it must use `glossary` directive
- no commonly known terms (TCP, WASM, etc) are defined
- no implementation details ("which uses TCP socket", ...). It applies **only** to glossary
- all terms must be in single form
- do not rename existing terms, unless user explicitly asked to

You can check for compilation errors with `ninja -v -C build genvm/docs`. Ignore errors in "python" (`.py` files). You must check compilation after completing a big task, and fix issues in code you generated

Rst documentation is available here https://www.sphinx-doc.org/en/master/usage/restructuredtext/directives.html
