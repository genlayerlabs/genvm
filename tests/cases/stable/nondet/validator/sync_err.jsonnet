local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/sync.py', 'main') {
	next: [super.next[0] {
		modes: 's',
		leader_nondet: [
			{
				"kind": "user_error",
				"value": "No idea"
			}
		]
	}],
}])}
