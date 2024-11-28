local simple = import '../../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/rollback.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
    leader_nondet: [
        {
            "kind": "rollback",
            "value": "other rollback"
        }
    ]
}
