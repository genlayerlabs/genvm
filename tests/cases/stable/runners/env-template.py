# {
#   "Seq": [
#     { "AddEnv": { "name": "X", "val": "1" } },
#     { "AddEnv": { "name": "X", "val": "${X}:2" } },
#     { "Depends": "py-genlayer:test" }
#   ]
# }

import os

print(os.environ['X'])
exit(0)
