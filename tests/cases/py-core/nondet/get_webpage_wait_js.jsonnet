local simple = import 'templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/get_webpage_wait_js.py') {
        "calldata": |||
            {
                "method": "main",
                "args": ["600ms"]
            }
        |||
    },
    simple.run('${jsonnetDir}/get_webpage_wait_js.py') {
        "calldata": |||
            {
                "method": "main",
                "args": ["0ms"]
            }
        |||
    }
]
