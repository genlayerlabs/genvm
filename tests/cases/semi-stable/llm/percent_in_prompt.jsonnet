local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/${fileBaseName}.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
    leader_nondet: [
        {
            "kind": "return",
            "value": "%0"
        }
    ]
}
