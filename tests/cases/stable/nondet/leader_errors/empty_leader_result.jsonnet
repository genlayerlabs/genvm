local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/simple.py', 'bar') {
	next: [super.next[0] {
		modes: 'vs',
		leader_nondet: [
			{
				"kind": "raw",
				"value": [],
			},
		],
	}],
}])}
