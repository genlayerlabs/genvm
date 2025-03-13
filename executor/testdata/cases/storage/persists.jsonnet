local simple = import 'templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/persists.py') {
        "calldata": |||
            {
                "method": "first",
                "args": []
            }
        |||
    },
    simple.run('${jsonnetDir}/persists.py') {
        "calldata": |||
            {
                "method": "second",
                "args": []
            }
        |||
    },
]
