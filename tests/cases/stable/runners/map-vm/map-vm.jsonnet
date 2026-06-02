local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{
	prepare: '${jsonnetDir}/prepare.py',
	entry: util.addPaths([simple_deploy.run('${jsonnetDir}/contract.zip') {stable_hash: false}])
}
