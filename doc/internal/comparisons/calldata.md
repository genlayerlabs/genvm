# Calldata size comparison


``` python
from pathlib import Path
import os
import inspect

script_dir = Path("__file__").parent.absolute()
root_dir = script_dir
while not root_dir.joinpath('.genvm-monorepo-root').exists():
    root_dir = root_dir.parent

import sys
sys.path.append(str(root_dir.joinpath('sdk-python', 'py')))
import genlayer.calldata as calldata
import rlp

import pandas as pd

ins = [
    10,
    10**9,
    10**18,
    10**32,
    10**100,
    [],
    b'',
    [[], [[]], [[], []]],
    'a',
    b'a',
    b'\xff',
    b'\x89\xc8\xa4\xc1A`r\x9e|\x93C\x05\xa9\x0c\xb7\xc2\xd8\xb7dV\xf0\xa2\x9enX\xf9[v\xaf/`\xffC\xe0\x08n\xe3\xcc\x82j\xbdI\xb1#E\x00:\xcc\x18\x19\x9e\xebf2\x82dO\x1eG_W\x17J@\xef\x15\x08\xd5NI\xe53\xddE\x8eMw\xfbtt\x81\xae3_\xa8C\x0c\xb0\xe3\x91\x1a\xa9*\t\xeeZ\xc7\xe7A\xef'
    b'',
    "abc",
    b"123",
    "й",
    "русский",
    [1, 1, 3],
    [1, '123123', 'b'],
    [0, 2, 3] * 10,
]
df = []
for x in ins:
    rlp_d = rlp.encode(x)
    genvm_d = calldata.encode(x)
    df.append((x, len(rlp_d), len(genvm_d)))

df = pd.DataFrame(df, columns=["input", "rlp size", "genvm calldata size"])

mean = (df["genvm calldata size"] - df["rlp size"]).mean()
mean_all = df["rlp size"].mean()

df
```

<div>
<style scoped>
    .dataframe tbody tr th:only-of-type {
        vertical-align: middle;
    }
&#10;    .dataframe tbody tr th {
        vertical-align: top;
    }
&#10;    .dataframe thead th {
        text-align: right;
    }
</style>

|  | input | rlp size | genvm calldata size |
|----|----|----|----|
| 0 | 10 | 1 | 1 |
| 1 | 1000000000 | 5 | 5 |
| 2 | 1000000000000000000 | 9 | 9 |
| 3 | 100000000000000000000000000000000 | 15 | 16 |
| 4 | 1000000000000000000000000000000000000000000000... | 43 | 48 |
| 5 | \[\] | 1 | 1 |
| 6 | b'' | 1 | 1 |
| 7 | \[\[\], \[\[\]\], \[\[\], \[\]\]\] | 7 | 7 |
| 8 | a | 1 | 2 |
| 9 | b'a' | 1 | 2 |
| 10 | b'\xff' | 2 | 2 |
| 11 | b'\x89\xc8\xa4\xc1A\`r\x9e\|\x93C\x05\xa9\x0c\xb... | 102 | 102 |
| 12 | abc | 4 | 4 |
| 13 | b'123' | 4 | 4 |
| 14 | й | 3 | 3 |
| 15 | русский | 15 | 15 |
| 16 | \[1, 1, 3\] | 4 | 4 |
| 17 | \[1, 123123, b\] | 10 | 11 |
| 18 | \[0, 2, 3, 0, 2, 3, 0, 2, 3, 0, 2, 3, 0, 2, 3, ... | 31 | 32 |

</div>

Mean difference is 0.5263157894736842 of 13.631578947368421 (bytes
favoring rlp). Which is because rlp doesn’t encode types, for instance

``` python
rlp.decode(rlp.encode(1234))
```

    b'\x04\xd2'
