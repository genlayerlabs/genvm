{
	"python": [
		{ "AddEnv": { "name": "pwd", "val": "/" } },
		{ "MapCode": { "to": "/contract.py" } },
		{ "MapFile": { "file": "${artifacts}/genvm-python-sdk.frozen", "to": "/sdk.frozen" } },
		{ "SetArgs": { "args": ["py", "-u", "-c", "import contract ; import genlayer.runner as r ; r.run(contract)"] } },
		{ "LinkWasm": { "file": "${artifacts}/softfloat.wasm" } },
		{ "StartWasm": { "file": "${artifacts}/genvm-python.wasm", "debug_path": "genvm-python.wasm" } }
	]
}
