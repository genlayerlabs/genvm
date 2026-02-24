local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/methods.py') {
	"calldata": |||
		{
			"method": "rback",
			"args": []
		}
	|||
}])}
