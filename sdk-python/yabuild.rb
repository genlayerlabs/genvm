project('sdk-python') {

	debug_sdk = false
	if config&.profile == "debug"
		debug_sdk = true
	end

	py_deps = Dir.glob(cur_src.join('py').to_s + "/**/*.py") + Dir.glob(cur_src.join('lib').to_s + "/**/*.py")
	genvm_python_raw = target_cargo_build(
		name: 'genvm-python',
		target: 'wasm32-wasi',
		profile: "release",
		features: if debug_sdk then ['sdk-debug'] else [] end
	)
	if not debug_sdk
		genvm_python_raw.add_deps(*py_deps)
	end

	genvm_python_out_patched = Pathname.new(genvm_python_raw.output_file).sub_ext('.patched.wasm')

	py_targets = []

	build_patcher = target_cargo_build(
		name: 'genvm-softfloat-patcher',
		dir: cur_src.parent.join('tools', 'softfloat-lib', 'patch-floats')
	)

	py_targets << target_command(
		output_file: genvm_python_out_patched,
		dependencies: [genvm_python_raw, build_patcher],
		command: [
			build_patcher.output_file,
			genvm_python_raw.output_file, genvm_python_out_patched
		]
	)

	if debug_sdk
		py_sdk_debug = target_command(
			output_file: cur_src.join('target', 'sdk.frozen'),
			dependencies: py_deps + [cur_src.join('src', 'build_debug_sdk.rs')],
			command: ['cargo', 'run', '--bin', 'genvm-python-build-debug-sdk', '--features', 'sdk-debug']
		)

		py_targets << py_sdk_debug
	end

	runner_target = target_publish_runner(
		name_base: 'genvm-rustpython',
		out_dir: config.runners_dir,
		files: [
			{ path: 'genvm-python.wasm', read_from: genvm_python_out_patched }
		]+ if debug_sdk then [{ path: 'genvm-python-sdk.frozen', read_from: py_sdk_debug.output_file }] else [] end,
		runner_dict: {
			"depends": [
				"softfloat:FC4YL2JHW76LFFWNIJZZ62D4Q6I5APZK2MUMQHNKNUS6E7DDZMS4FOYG3YUQ4MIAR4N4XR5JLXRI5RBWZ6BQHFT5V2MRCDV34LBE7NI="
			],
			"actions": [
				{ "AddEnv": { "name": "pwd", "val": "/" } },
				{ "MapCode": { "to": "/contract.py" } },
			] + if config.profile == "debug" then [{ "MapFile": { "file": "genvm-python-sdk.frozen", "to": "/sdk.frozen" } }] else [] end + [
				{ "SetArgs": { "args": ["py", "-u", "-c", "import contract ; import genlayer.runner as r ; r.run(contract)"] } },
				{ "StartWasm": { "file": "genvm-python.wasm" } }
			]
		}
	)

	target_alias(
		'all',
		runner_target,
		tags: ['all']
	)
}
