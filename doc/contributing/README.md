# Contributing to GenVM development

## Sub pages
- [running tests](./tests.md)
- [default workflows](./workflows.md)

## PR requirements
Main requirement is that all tests must pass. It includes [pre-commit](https://pre-commit.com) and test suites. PRs are merged via a queue, that executes all tests and merges *iff* they all pass. It is done to ensure that `HEAD` of `main` branch is always stable
