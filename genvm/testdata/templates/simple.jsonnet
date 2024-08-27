{
	run(scriptfile)::
		{
			"vars": {},
			"runners": {
				"python": [
					{ "AddEnv": { "name": "pwd", "val": "/" } },
					{ "MapCode": { "to": "/contract.py" } },
					{ "MapFile": { "file": "${artifacts}/genvm-python-sdk.frozen", "to": "/sdk.frozen" } },
					{ "SetArgs": { "args": ["py", "contract.py"] } },
					{ "LinkWasm": { "file": "${artifacts}/softfloat.wasm" } },
					{ "StartWasm": { "file": "${artifacts}/genvm-python.wasm", "debug_path": "genvm-python.wasm" } }
				]
			},

			"accounts": {
				"AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=": {
					"code": scriptfile
				},
				"AgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=": {
					"code": null
				}
			},

			"message": {
				"gas": 9007199254740991,
				"contract_account": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
				"sender_account": "AgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
				"value": null
			},

			"calldata": "{}"
		}
}
