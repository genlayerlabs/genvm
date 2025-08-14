local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/${fileBaseName}.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
    sync: true,
    leader_nondet: [
        {
            "kind": "return",
            "value": "123"
        }
    ]
}
