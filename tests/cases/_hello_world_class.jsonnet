local simple = import 'templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/${fileBaseName}.py') {
        "calldata": |||
            {
                "method": "foo",
                "args": []
            }
        |||
    },
    simple.run('${jsonnetDir}/${fileBaseName}.py') {
        "calldata": |||
            {
                "method": "foo",
                "args": []
            }
        |||
    }
]
