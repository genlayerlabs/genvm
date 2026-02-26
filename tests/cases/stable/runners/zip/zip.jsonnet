local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{
	prepare: '${jsonnetDir}/prepare.py',
	entry: util.addPaths(util.mapGraph(function(e) e {stable_hash: false}, [simple.run('${jsonnetDir}/contract.zip', 'foo')]))
}
