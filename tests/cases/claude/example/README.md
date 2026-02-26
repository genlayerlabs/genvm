create a sample .py and .jsonnet in tests/cases/stable/claude/example for following: deploy A and B, then emit write method to B that will read A.
Write jsonnet yourself: don't use simple.jsonnet but do use util.lib

For all transactions specify:
1. semantics: []
2. modes: lvs
3. stable_hash: false

For top-level statement specify tag "fuzz"
