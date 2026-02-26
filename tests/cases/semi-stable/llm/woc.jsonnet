local simple = import 'templates/simple_deploy.jsonnet';
local msg = import 'templates/message.json';
local s = simple.run('${jsonnetDir}/${fileBaseName}.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"args": [True]
			}
		|||,
	},

	s {
		code: null,
		message: msg,
		"calldata": |||
			{
				"method": "get_have_coin",
			}
		|||,
	},

	s {
		code: null,
		message: msg,
		"calldata": |||
			{
				"method": "ask_for_coin",
				"args": ["pwease"],
			}
		|||,
		stable_hash: false,
	},

	s {
		code: null,
		message: msg,
		"calldata": |||
			{
				"method": "get_have_coin",
			}
		|||,
	},
])])}
