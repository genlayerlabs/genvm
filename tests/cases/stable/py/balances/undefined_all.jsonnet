local simple = import 'templates/simple_deploy.jsonnet';
local msg = import 'templates/message.json';
local s = simple.run('${jsonnetDir}/undefined_all.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"method": "main",
				"args": [],
			}
		|||,
		message: s.message {
			"value": 100,
		}
	},
	s {
		code: null,
		message: msg,
		"calldata": |||
			{
				"method": "main",
				"args": [],
			}
		|||,
	},
	s {
		code: null,
		message: msg {
			"value": 100,
		},
		"calldata": |||
			{}
		|||,
	},
])])}
