local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/rollback.py') {
	"calldata": |||
		{
			"method": "main",
			"args": []
		}
	|||,
	leader_nondet: [
		{
			"kind": "rollback",
			"value": "rollback"
		}
	]
}])}
