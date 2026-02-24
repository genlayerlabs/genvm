local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/sync.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
    modes: 's',
    leader_nondet: [
        {
            "kind": "rollback",
            "value": "No idea"
        }
    ]
}])}
