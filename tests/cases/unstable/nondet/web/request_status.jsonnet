local simple = import 'templates/simple_deploy.jsonnet';
local msg = import 'templates/message.json';
local s = simple.run('${jsonnetDir}/request_status.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"method": "main",
				"args": [200]
			}
		|||,
		stable_hash: false,
	},
	s {
		code: null,
		message: msg,
		"calldata": |||
			{
				"method": "main",
				"args": [404]
			}
		|||,
		stable_hash: false,
	},
])])}
