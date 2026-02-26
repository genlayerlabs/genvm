local msg = import 'templates/message.json';
local util = import 'templates/util.jsonnet';

local addrA = "AQAAAAAAAAAAAAAAAAAAAAAAAAA=";
local addrB = "AwAAAAAAAAAAAAAAAAAAAAAAAAA=";
local sender = "AgAAAAAAAAAAAAAAAAAAAAAAAAA=";

local vars = {
	fromAddr: addrA,
	toAddr: addrB,
};

local common = {
	expected_semantics_components: [],
	modes: 'lvs',
	stable_hash: false,
};

{
	tags: ["fuzz"],
	entry: util.addPaths([
		// step 0: deploy A
		common {
			vars: vars,
			deadline: 10,
			code: '${jsonnetDir}/contract_a.py',
			message: msg {
				contract_address: addrA,
				is_init: true,
			},
			calldata: '{}',

			next: [
				// step 1: deploy B
				common {
					vars: vars,
					deadline: 10,
					code: '${jsonnetDir}/contract_b.py',
					message: msg {
						contract_address: addrB,
						is_init: true,
					},
					calldata: '{}',

					next: [
						// step 2: call B.read_from(A)
						common {
							vars: vars,
							deadline: 10,
							code: null,
							accounts: {
								[addrA]: { code: '${jsonnetDir}/contract_a.py' },
								[addrB]: { code: '${jsonnetDir}/contract_b.py' },
								[sender]: { code: null },
							},
							message: msg {
								contract_address: addrB,
							},
							calldata: |||
								{
									"method": "read_from",
									"args": [Address(fromAddr)]
								}
							|||,
						},
					],
				},
			],
		},
	]),
}
