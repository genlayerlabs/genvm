local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{
	prepare: '${jsonnetDir}/prepare.py',
	entry: util.addPaths([simple.run('${jsonnetDir}/contract.zip') {
    "calldata": |||
        {
            "method": "foo",
            "args": []
        }
    |||
}])
}
