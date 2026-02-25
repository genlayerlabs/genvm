local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([
	simple.run('${jsonnetDir}/${fileBaseName}.py', 'main', [idx])
	for idx in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
])}
