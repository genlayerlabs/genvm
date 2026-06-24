local two = import 'templates/two.jsonnet';
local msg = import 'templates/message.json';
local util = import 'templates/util.jsonnet';

// every entry point that reads a contract's (mismatched) major must reject it:
local entries = [
	// 0: top-level call to a major-mismatched contract
	{
		vars: {},
		code: '${jsonnetDir}/target.py',
		message: msg + {is_init: true},
		calldata: '{}',
		next: [{
			vars: {},
			code: null,
			message: msg,
			calldata: std.manifestJsonEx({method: 'foo', args: []}, '    '),
		}],
	},
	// 1: CallContract into a major-mismatched contract
	two.run('${jsonnetDir}/caller.py', '${jsonnetDir}/target.py',
		|||
			{
				"method": "call",
				"args": [Address(toAddr)]
			}
		|||
	),
	// 2: map_file from a major-mismatched contract
	two.run('${jsonnetDir}/caller.py', '${jsonnetDir}/target.py',
		|||
			{
				"method": "map",
				"args": [Address(toAddr)]
			}
		|||
	),
];

{entry: util.addPaths(util.mapGraph(function(e) e + {stable_hash: false}, entries))}
