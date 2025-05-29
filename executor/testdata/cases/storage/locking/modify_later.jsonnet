local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/code.py');
[
    s {
        "calldata": |||
            {
                "args": [[Address(b'\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00')], False],
            }
        |||,
        message: super.message + {
            "is_init": true,
        },
    },
    s {
        "calldata": |||
            {
                "method": "try_modify",
            }
        |||
    },
    s {
        "calldata": |||
            {
                "method": "nop",
            }
        |||
    }
]
