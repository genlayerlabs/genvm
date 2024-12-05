local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/sync.py') {
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
