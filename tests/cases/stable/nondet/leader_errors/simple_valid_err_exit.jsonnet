local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/simple.py', 'ex') {
	next: [super.next[0] {
		leader_nondet: [
			{
				"kind": "contract_error",
				"value": "exit_code 2"
			}
		],
	}],
}])}
