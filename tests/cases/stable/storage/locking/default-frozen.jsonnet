local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/code.py');
[
    s {
        "calldata": |||
            {
                "args": [[], False],
            }
        |||,
        message: super.message + {
            "is_init": true,
        },
    },
    s {
        code: null,
        "calldata": |||
            {
                "method": "try_modify",
            }
        |||
    },
    s {
        code: null,
        "calldata": |||
            {
                "method": "nop",
            }
        |||
    }
]
