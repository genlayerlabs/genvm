local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/code.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"args": [[], False],
			}
		|||,
		message: super.message + {
			"is_init": true,
		},
	},
	s {
		code: null,
		"calldata": |||
			{
				"method": "try_modify",
			}
		|||
	},
	s {
		code: null,
		"calldata": |||
			{
				"method": "nop",
			}
		|||
	}
])])}
