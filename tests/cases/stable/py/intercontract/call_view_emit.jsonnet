local simple = import 'templates/two.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/call_view_emit_from.py', '${jsonnetDir}/call_view_emit_to.py',
	|||
		{
			"method": "main",
			"args": [Address(toAddr)]
		}
	|||
)])}
