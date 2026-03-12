local simple = import 'templates/two.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/calldata_rollback_from.py', '${jsonnetDir}/calldata_rollback_to.py', |||
		{
			"method": "main",
			"args": [Address(toAddr)]
		}
	|||
)])}
