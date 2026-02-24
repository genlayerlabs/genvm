local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/../web/get_webpage.py') {
	"calldata": |||
		{
			"method": "main",
			"args": ["text"]
		}
	|||,
	leader_nondet: [
		{
			"kind": "return",
			"value": "Hello world~"
		}
	]
}])}
