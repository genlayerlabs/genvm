local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/ret-tuple.py') {
	"calldata": |||
		{
			"method": "#get-schema"
		}
	|||
}])}
