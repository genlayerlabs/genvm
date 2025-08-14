local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/send_message_on.py');
[
    s {
        "calldata": |||
            {
                "method": "main",
                "args": ["finalized"],
            }
        |||
    },
        s {
        "calldata": |||
            {
                "method": "main",
                "args": ["accepted"],
            }
        |||
    },
        s {
        "calldata": |||
            {
                "method": "main",
                "args": ["random"],
            }
        |||
    },
]
