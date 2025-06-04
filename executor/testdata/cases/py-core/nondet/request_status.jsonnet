local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/request_status.py');
[
    s {
        "calldata": |||
            {
                "method": "main",
                "args": [200]
            }
        |||
    },
    s {
        "calldata": |||
            {
                "method": "main",
                "args": [404]
            }
        |||
    },
]
