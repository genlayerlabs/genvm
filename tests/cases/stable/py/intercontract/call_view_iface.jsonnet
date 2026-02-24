local simple = import 'templates/two.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/call_view_from_iface.py', '${jsonnetDir}/call_view_to.py',
	|||
		{
			"method": "main",
			"args": [Address(toAddr)]
		}
	|||
)])}
