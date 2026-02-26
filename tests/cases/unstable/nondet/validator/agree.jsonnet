local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/../web/get_webpage.py', 'main', ["text"]) {
	next: [super.next[0] {
		leader_nondet: [
			{
				"kind": "return",
				"value": "Hello world!"
			}
		],
		modes: 'vs',
		stable_hash: true,
	}],
}])}
