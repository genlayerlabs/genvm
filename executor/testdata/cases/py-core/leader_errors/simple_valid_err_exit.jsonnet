local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/simple.py') {
    "calldata": |||
        {
            "method": "ex",
            "args": []
        }
    |||,
    leader_nondet: [
        {
            "kind": "contract_error",
            "value": "leader_err"
        }
    ],
}
