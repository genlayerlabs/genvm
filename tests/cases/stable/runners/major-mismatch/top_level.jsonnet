local msg = import 'templates/message.json';
local util = import 'templates/util.jsonnet';
{
	entry: util.addPaths([{
		vars: {},
		code: '${jsonnetDir}/target.py',
		message: msg + {is_init: true},
		calldata: '{}',
		stable_hash: false,
		next: [{
			vars: {},
			code: null,
			message: msg,
			calldata: std.manifestJsonEx({method: 'foo', args: []}, '    '),
			stable_hash: false,
		}],
	}])
}
