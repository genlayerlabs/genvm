local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/code.py');
[
    s {
        "calldata": |||
            {
                "args": [[], True],
            }
        |||,
        message: super.message + {
            "is_init": true,
        },
    },
    s {
        "calldata": |||
            {
                "method": "nop",
            }
        |||
    }
]
