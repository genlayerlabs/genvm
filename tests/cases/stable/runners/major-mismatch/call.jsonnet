local two = import 'templates/two.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([two.run('${jsonnetDir}/caller.py', '${jsonnetDir}/target.py',
	|||
		{
			"method": "call",
			"args": [Address(toAddr)]
		}
	|||
)])}
