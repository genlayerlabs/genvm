local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/${fileBaseName}.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([
    s {
        "calldata": std.format(|||
            {
                "method": "main",
                "args": [%d]
            }
        |||, idx)
    }
    for idx in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
])}
