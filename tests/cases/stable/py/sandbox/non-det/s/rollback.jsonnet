local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/../code.py') {
	"calldata": |||
		{
			"method": "main",
			"args": ["gl.advanced.user_error_immediate('RB')"]
		}
	|||
}])}
