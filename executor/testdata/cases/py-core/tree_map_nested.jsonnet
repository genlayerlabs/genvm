local simple = import 'templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/tree_map_nested.py') {
        "calldata": |||
            {
                "method": "foo",
                "args": []
            }
        |||
    },
]
