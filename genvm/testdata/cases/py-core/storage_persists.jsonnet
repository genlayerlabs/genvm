local simple = import '../../templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/storage_persists.py') {
        "calldata": |||
            {
                "method": "first",
                "args": []
            }
        |||
    },
    simple.run('${jsonnetDir}/storage_persists.py') {
        "calldata": |||
            {
                "method": "second",
                "args": []
            }
        |||
    },
]
