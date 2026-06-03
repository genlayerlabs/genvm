---
name: pydoc
description: Add RST docstrings to public Python methods/classes. Use when asked to document Python code.
argument-hint: "[file.py ...]"
---

# Python Documentation Skill

Add RST-format docstrings to public entities in the specified Python files.

## Rules

1. **Public only**: Document classes, methods, functions, and properties that do NOT start with `_`. Skip private/internal entities entirely.
2. **RST format**: Use reStructuredText docstring syntax (`:param:`, `:returns:`, `:raises:`, `:type:`, etc.).
3. **Concise**: One short sentence for the summary. No filler, no restating the obvious from the signature. If the method name is self-explanatory (e.g., `clear`, `append`), the summary can be very brief.
4. **Document exceptions**: Always include `:raises ExceptionType:` for every exception the method can raise (including those from called helpers like `_map_index`). Check the implementation, not just the signature.
5. **Do not document**: `__init__` that just raises TypeError (already has a docstring by convention), or dunder methods whose behavior is obvious from the protocol they implement (e.g., `__len__`, `__iter__`, `__repr__`, `__contains__`) — unless they have non-obvious behavior or raise specific exceptions worth noting.
6. **Preserve existing docstrings**: If a method already has a docstring, update it to match these rules only if it is incomplete (e.g., missing `:raises:`). Do not rewrite correct docstrings.
7. **No type annotations in docstrings**: Types are already in the signature; do not duplicate them with `:type:` or `:rtype:` fields.

## Format Template

The opening `"""` goes on its own line, followed by the summary on the next line:

```python
def method(self, param: Type) -> ReturnType:
    """
    Short summary of what this does.

    :param param: what this parameter means
    :returns: what is returned
    :raises ValueError: when param is invalid
    """
```

For multi-line descriptions (rare — prefer single line):

```python
def method(self) -> ReturnType:
    """
    Short summary.

    Additional context only if the summary is insufficient.

    :returns: description
    :raises KeyError: when key is not found
    """
```

## Procedure

1. Read the file(s) passed as arguments (ARGUMENTS section below).
2. Identify all public entities without docstrings or with incomplete docstrings.
3. For each, trace through the implementation to find all possible exceptions.
4. Add or update docstrings using the Edit tool. Do not rewrite the file.
5. Do NOT add docstrings to private methods (starting with `_`), internal descriptor classes, or non-public helpers.

ARGUMENTS: $ARGUMENTS
