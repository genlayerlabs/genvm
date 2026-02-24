local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/get_webpage_screenshot.py') {
	"calldata": |||
		{
			"method": "main",
			"args": ["text"]
		}
	|||,
	stable_hash: false,
}])}
