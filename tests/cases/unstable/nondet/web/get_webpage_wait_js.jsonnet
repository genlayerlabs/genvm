local simple = import 'templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/get_webpage_wait_js.py') {
        "calldata": |||
            {
                "method": "main",
                "args": ["15s"]
            }
        |||,
        deadline: 60,
    },
    simple.run('${jsonnetDir}/get_webpage_wait_js.py') {
        "calldata": |||
            {
                "method": "main",
                "args": ["0ms"]
            }
        |||,
        deadline: 60,
    }
]
