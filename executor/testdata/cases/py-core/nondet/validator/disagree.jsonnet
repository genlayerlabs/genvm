local simple = import '../../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/../get_webpage.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["text"]
        }
    |||,
    leader_nondet: [
        {
            "ok": true,
            "value": "Hello world~"
        }
    ]
}
