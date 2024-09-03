{
	"python": [
		{ "AddEnv": { "name": "pwd", "val": "/" } },
		{ "MapCode": { "to": "/contract.py" } },
		{ "MapFile": { "file": "${artifacts}/wasm/genvm-python-sdk.frozen", "to": "/sdk.frozen" } },
		{ "SetArgs": { "args": ["py", "-u", "-c", "import contract ; import genlayer.runner as r ; r.run(contract)"] } },
		{ "LinkWasm": { "file": "${artifacts}/wasm/softfloat.wasm" } },
		{ "StartWasm": { "file": "${artifacts}/wasm/genvm-python.wasm", "debug_path": "genvm-python.wasm" } }
	]
}
