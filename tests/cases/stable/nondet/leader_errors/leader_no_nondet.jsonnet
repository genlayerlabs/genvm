local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/simple.py') {
    "calldata": |||
        {
            "method": "bar",
            "args": []
        }
    |||,
    leader_nondet: [],
}])}
