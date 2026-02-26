local simple = import 'templates/simple_deploy.jsonnet';
local msg = import 'templates/message.json';
local s = simple.run('${jsonnetDir}/get_webpage_wait_js.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"method": "main",
				"args": ["15s"]
			}
		|||,
		deadline: 60,
		stable_hash: false,
	},
	s {
		code: null,
		message: msg,
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
