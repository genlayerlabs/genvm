local simple = import 'templates/simple_deploy.jsonnet';
local msg = import 'templates/message.json';
local s = simple.run('${jsonnetDir}/send_message_on.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"method": "main",
				"args": ["finalized"],
			}
		|||
	},
	s {
		code: null,
		message: msg,
		"calldata": |||
			{
				"method": "main",
				"args": ["accepted"],
			}
		|||
	},
	s {
		code: null,
		message: msg,
		"calldata": |||
			{
				"method": "main",
				"args": ["random"],
			}
		|||
	},
])])}
