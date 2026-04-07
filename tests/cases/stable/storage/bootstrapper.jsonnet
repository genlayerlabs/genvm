local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/${fileBaseName}.py');
local util = import 'templates/util.jsonnet';
local target_code = importstr './bootstrapper_target.py';

// contract instance slot = sha3_256(ROOT_SLOT_ID(32 zero bytes) + CONTRACT_OFFSET(0 as u32 le))
local contract_instance_slot = '372d46c3ada9f897c74d349bbfe0e450c798167c9f580f8daf85def57e96c3ea';

{entry: util.addPaths([util.chain([
	// deploy bootstrapper
	s {
		message: super.message + {"is_init": true},
	},
	// write i32 value 42 to the contract instance slot at offset 0
	s {
		code: null,
		calldata: |||
			{"method": "write", "args": [[(bytes.fromhex('%(slot)s'), 0, (42).to_bytes(4, 'little'))]]}
		||| % {slot: contract_instance_slot},
	},
	// push first half of target contract code
	s {
		code: null,
		vars: {code: target_code},
		calldata: |||
			{"method": "push_code", "args": [code[:len(code)//2].encode()]}
		|||,
	},
	// push second half of target contract code
	s {
		code: null,
		vars: {code: target_code},
		calldata: |||
			{"method": "push_code", "args": [code[len(code)//2:].encode()]}
		|||,
	},
	// finish bootstrapping (copies code from temp slot to code slot)
	s {
		code: null,
		calldata: |||
			{"method": "finish"}
		|||,
	},
	// call get_field on the now-bootstrapped contract
	s {
		code: null,
		calldata: |||
			{"method": "get_field"}
		|||,
	},
])])}
