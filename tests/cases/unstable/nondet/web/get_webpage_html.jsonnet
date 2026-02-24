local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/get_webpage.py') {
	"calldata": |||
		{
			"method": "main",
			"args": ["html"]
		}
	|||
}])}
