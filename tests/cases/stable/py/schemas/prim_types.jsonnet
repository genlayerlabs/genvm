local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/prim_types.py') {
    "calldata": |||
        {
            "method": "#get-schema"
        }
    |||
}])}
