local msg = import 'templates/message.json';
local util = import 'templates/util.jsonnet';

local addr = "AQAAAAAAAAAAAAAAAAAAAAAAAAA=";

local common = {
	expected_semantics_components: [],
	modes: 'lvs',
	stable_hash: false,
};

{
	tags: ["fuzz"],
	entry: util.addPaths([
		// step 0: deploy
		common {
			deadline: 10,
			code: '${jsonnetDir}/contract.py',
			message: msg {
				contract_address: addr,
				is_init: true,
			},
			calldata: '{}',

			next: [
				// step 1: call compute() — leader/validator/sync hashes must agree
				common {
					deadline: 10,
					code: null,
					accounts: {
						[addr]: { code: '${jsonnetDir}/contract.py' },
					},
					message: msg {
						contract_address: addr,
					},
					calldata: |||
						{
							"method": "compute",
							"args": []
						}
					|||,
				},
			],
		},
	]),
}
