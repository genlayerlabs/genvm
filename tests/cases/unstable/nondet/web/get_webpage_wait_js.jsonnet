local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	simple.run('${jsonnetDir}/get_webpage_wait_js.py') {
		"calldata": |||
			{
				"method": "main",
				"args": ["15s"]
			}
		|||,
		deadline: 60,
		stable_hash: false,
	},
	simple.run('${jsonnetDir}/get_webpage_wait_js.py') {
		"calldata": |||
			{
				"method": "main",
				"args": ["0ms"]
			}
		|||,
		deadline: 60,
		stable_hash: false,
	}
])])}
