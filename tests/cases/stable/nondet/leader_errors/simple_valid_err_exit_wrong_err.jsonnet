local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/simple.py') {
	"calldata": |||
		{
			"method": "ex",
			"args": []
		}
	|||,
	leader_nondet: [
		{
			"kind": "rollback",
			"value": "exit_code 1"
		}
	],
}])}
