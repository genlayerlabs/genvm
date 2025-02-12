local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/undefined_method_payable.py');
[
    s {
        "calldata": |||
            {
                "method": "main",
                "args": [],
            }
        |||,
        message: s.message {
            "value": 100,
        }
    },
    s {
        "calldata": |||
            {
                "method": "main",
                "args": [],
            }
        |||,
    },
    s {
        "calldata": |||
            {}
        |||,
        message: s.message {
            "value": 100,
        }
    },
]
