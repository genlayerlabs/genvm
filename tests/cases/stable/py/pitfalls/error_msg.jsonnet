local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/error_msg.py') {
    "calldata": |||
        {
            "method": "#error"
        }
    |||
}])}
