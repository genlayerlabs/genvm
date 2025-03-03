---
name: Bug report
about: Create a report to help us improve
title: 'bug: *description*'
labels: bug
assignees: ''

---

*A clear and concise description of what the bug is.*

## Reproduction
### Contract code
```py
# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	...
```
### Calls sequence
1. Deploy *(no arguments)*
2. *Call write method `bar("123")`*
3. *Call read method `foo()`*

### Expected result
```
A
```
### Observed result
```
B
```

## Data
- GenVM version: *specify from genvm_log, eg 0.0.10*

## Additional informations

*Include additional files if necessary, such as custom `genvm-config.json`*
