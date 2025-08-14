local simple = import 'templates/simple.jsonnet';
[
    simple.run('${jsonnetDir}/state_in_nondet.py') {
        "calldata": |||
            {
                "method": "plain",
                "args": []
            }
        |||
    },
    simple.run('${jsonnetDir}/state_in_nondet.py') {
        "calldata": |||
            {
                "method": "generic",
                "args": []
            }
        |||
    }
]
