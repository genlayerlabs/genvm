local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/${fileBaseName}.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"args": [True]
			}
		|||,
		"message": super.message + {
			"is_init": true,
		}
	},

	s {
		"calldata": |||
			{
				"method": "get_have_coin",
			}
		|||,
	},

	s {
		"calldata": |||
			{
				"method": "ask_for_coin",
				"args": ["pwease"],
			}
		|||,
	},

	s {
		"calldata": |||
			{
				"method": "get_have_coin",
			}
		|||,
	},
])])}
