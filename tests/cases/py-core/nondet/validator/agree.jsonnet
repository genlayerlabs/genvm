local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../get_webpage.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["text"]
        }
    |||,
    leader_nondet: [
        {
            "kind": "return",
            "value": "Hello world!"
        }
    ]
}
