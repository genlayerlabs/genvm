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

	runner_dict = {
		"depends": [],
		"actions": [
			{ "AddEnv": { "name": "pwd", "val": "/" } },
			{ "MapCode": { "to": "/contract.py" } },
		] + if config.profile == "debug" then [{ "MapFile": { "file": "genvm-python-sdk.frozen", "to": "/sdk.frozen" } }] else [] end + [
			{ "SetArgs": { "args": ["py", "-u", "-c", "import contract ; import genlayer.runner as r ; r.run(contract)"] } },
			{ "LinkWasm": { "file": "softfloat.wasm" } },
			{ "StartWasm": { "file": "genvm-python.wasm" } }
		]
	}

	runner_json = cur_build.join('runner.json')
	File.write(runner_json, JSON.dump(runner_dict))

	softfloat_target = find_target('genvm/softfloat/lib')

	runner_publish_config = {
		"create_test_runner" => config.createTestRunner,
		"out_dir" => config.out_dir.join('share', 'genvm', 'runners', 'genvm-rustpython'),
		"files" => [
				{ "path" => 'genvm-python.wasm', "read_from" => genvm_python_out_patched },
				{ "path" => 'runner.json', "read_from" => runner_json },
				{ "path" => 'softfloat.wasm', "read_from" => find_target('genvm/softfloat/lib').meta.output_file }
		] + if debug_sdk then [{ "path" => 'genvm-python-sdk.frozen', "read_from" => py_sdk_debug.output_file }] else [] end
	}

	fake_out = cur_build.join('published-runner.trg')
	dep_file = fake_out.sub_ext('.d')
	runner_publish_config['dep_file'] = dep_file
	runner_publish_config['fake_out'] = fake_out
	publish_script = root_src.join('build-scripts', 'publish-runner.py')
	runner_target = target_command(
		output_file: fake_out,
		commands: [
			[publish_script, JSON.dump(runner_publish_config)],
			['touch', fake_out]
		],
		depfile: dep_file,
		dependencies: [publish_script, softfloat_target] + py_targets
	)

	all.add_deps(
		target_alias(
			'all',
			runner_target
		)
	)
}
