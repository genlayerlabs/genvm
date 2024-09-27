# Should the contract be a class?

<table>
<tr><td>Criteria</td><td>Class</td><td>Functions</td></tr>
<tr>
<td>code</td>
<td>

```py
class Contract(IContract):
  deployer: Address



  def __init__(self):
    self.deployer = message.sender_account

  @public
  def foo(self):
    self.bar()
  def bar(self):
    pass
```

</td>

<td>

```py
class Storage:
  deployer: Address

storage = genlayer.get_storage(Storage)

def init():
  storage.deployer = message.sender_account

@public
def foo():
  bar()
def bar():
  pass
```

</td>

</tr>
<tr style="vertical-align: top;"><td>Pros</td><td>

1. It is more obvious that `self.` is a call to contract method.
2. Functions without access to `self` don't see storage implicitly

</td><td>

1. Much less symbols

</td></tr>
<tr style="vertical-align: top;"><td>Cons</td><td>

1. Is it obvious that all class fields are in the storage?
2. Is it obvious that no new fields should be added via `self.x =`?

</td><td>

1. more `C` like

</td></tr>
</table>

# Equivalence principle

## Questions that drive the API
1. Should we provide users with the ability to know if they are the leader?
2. Should we provide them with leader's result?

Is there a security risk from giving this info to the user? In general, no:
1. nondeterministic block can call random (even if not local, from random dot org)
2. this will introduce difference only between leader and validators
So, unless leader gets some score for agreeing or disagreeing with validators it is "safe"

Basis function:
```rust
fn eq_principle(leader_fn: () -> bytes|rollback, validator_fn: (leaders_result: bytes|rollback) -> bool) -> bytes|rollback
```
On top of it all sorts of API can be built, including `eq_principle(prompt_template, result_producer)`

### Examples
```py
PROMPT_TEMPLATE = """
{user_prompt}

{leaders}

{mine}

"""

def eq_principle_prompt(prompt, func):
    def leader_fn():
        return fn()
    def validator_fn(leader_res):
        my_res = fn()
        return "yes" in call_llm(PROMPT_TEMPLATE.format(user_prompt=prompt, leaders=leader_res, mine=my_res))
    return eq_principle(leader_fn, validator_fn)

def fn():
    pass

eq_principle_prompt("The score and the winner has to be exactly the same", fn)
```

```py
def leader_fn():
    web = get_webpage()
    return call_llm('summary of {web}')
def validator_fn(leader_res):
    web = get_webpage()
    return "yes" in call_llm('summary of {web} is {leaders}')
return eq_principle(leader_fn, validator_fn)
```

## Isolation

Problem statement: all "captures" must be serialized. Necessary storage variables must be read in advance and passed as well

### Why nondeterministic block can't read storage?
That will make async optimization unsafe:
```py
            storage_write *x, 1

# thread 1                 # thread 2
start_non_det_async     |
storage_write *x, 2     |
                        |  storage_read *x # => 2
```

## User interface
Rhere is no multiline lambda in python. Creator of python suggests following approach
```py
def contract_method():
  def nondet_block():
    return get_webpage("https://example.org")
  return eq_principle_refl(nondet_block)
```

Unfortunately, we can't extract a block from `with` statement without patching python

## Serialization libraries
Example of use of `dill`
```py
x = 1

class B:
  def __init__(self):
    global x
    x = 2
    self.b = 10
  def foo(self, loc):
    def bar():
      return self.b + x + loc
    return bar
import dill
def ser():
  o = B()
  print(dill.dumps(o.foo(100), recurse=True)) # without `recurse=True` x will be reset to 1 in new VM
```
Cons:
All classes involved are serialized

Note: there is also `cloudpickle` package

# Naming
How are we going to name modules?
1. top level package `genlayer`
2. pure python package `genlayer.py` (it doesn't require GenVM to be imported)
3. top level package that requires GenVM `genlayer.sdk` ~~or `genlayer.genvm`~~
    - note: it can't be in `__init__.py`: otherwise it will be loaded with `genlayer.py` and fail
