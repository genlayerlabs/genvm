local simple = import 'templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/${fileBaseName}.py') {
        "calldata": |||
            {
                "method": "first",
                "args": []
            }
        |||
    },
    simple.run('${jsonnetDir}/${fileBaseName}.py') {
        "calldata": |||
            {
                "method": "second",
                "args": []
            }
        |||
    },
]
