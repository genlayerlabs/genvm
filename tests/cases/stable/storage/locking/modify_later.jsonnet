local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/code.py');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
	s {
		"calldata": |||
			{
				"args": [[Address(b'\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00')], False],
			}
		|||,
		message: super.message + {
			"is_init": true,
		},
	},
	s {
		"calldata": |||
			{
				"method": "try_modify",
			}
		|||,
		code: null,
	},
	s {
		"calldata": |||
			{
				"method": "nop",
			}
		|||,
		code: null,
	}
])])}
