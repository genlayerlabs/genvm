local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/gvm-89.py') {
	"calldata": |||
		{
			"method": "main"
		}
	|||
}])}
