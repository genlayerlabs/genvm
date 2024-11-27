# What should be passed to functions that boil down to prompts?

1. any
2. string

There are three ways in python to coerce value to a string in python:
1. `str` (which calls `__str__`)
2. `repr` (which calls `__repr__`)
3. `str.format` (which calls either above or `__format__`); [pep 3101](https://peps.python.org/pep-3101/)

f-string is essentially `str.format` as it follows same rules; [pep 498](https://peps.python.org/pep-0498/)

## Examples from python:

```python
>>> '1' + 2
Traceback (most recent call last):
  File "<stdin>", line 1, in <module>
TypeError: can only concatenate str (not "int") to str

>>> f'{object()}'
'<object object at 0x7fa761e30740>'

>>> str("123")
'123'
>>> repr("123")
"'123'"

>>> '{:.5f}'.format(11)
'11.00000'
>>> '{:}'.format(11)
'11'

>>> str({"x": 11})
"{'x': 11}"
```

## Why we shouldn't decide for users which method to use?

1. there are three default methods
2. this way we don't control precision
3. `str(dict)` produces invalid json (single quotes inside of double)
4. Python Zen says
    > Explicit is better than implicit.
5. writing `return str(x)` has the same writer complexity as writing `return x`
