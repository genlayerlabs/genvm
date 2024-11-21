# { "Depends": "py-genlayer:test" }

import re

assert re.match('(ab|ba)', 'ab').span() == (0, 2)
assert re.match('(ab|ba)', 'ba').span() == (0, 2)
assert re.match('(abc|bac|ca|cb)', 'abc').span() == (0, 3)
assert re.match('(abc|bac|ca|cb)', 'bac').span() == (0, 3)
assert re.match('(abc|bac|ca|cb)', 'ca').span() == (0, 2)
assert re.match('(abc|bac|ca|cb)', 'cb').span() == (0, 2)
assert re.match('((a)|(b)|(c))', 'a').span() == (0, 1)
assert re.match('((a)|(b)|(c))', 'b').span() == (0, 1)
assert re.match('((a)|(b)|(c))', 'c').span() == (0, 1)

exit(0)
