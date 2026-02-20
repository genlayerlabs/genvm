local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/${fileBaseName}.py') {
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
}])}
