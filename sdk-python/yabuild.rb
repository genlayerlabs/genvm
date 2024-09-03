project('sdk-python') {

	debug_sdk = config.sdkPython&.debugSdk
	if debug_sdk.nil?
		debug_sdk = false
	end

	out_raw = cur_src.join('target', 'wasm32-wasi', 'release', 'genvm-python.wasm')
	deps = ['src/main.rs', 'src/pyimpl.rs']
	py_deps = Dir.glob(cur_src.join('py').to_s + "/**/*.py") + Dir.glob(cur_src.join('lib').to_s + "/**/*.py")
	if not debug_sdk
		deps += py_deps
	end
	deps = deps.map { |f| cur_src.join(f) }
	target_command(
		output_file: out_raw,
		dependencies: deps,
		command: [
			'cargo', 'build',
			'--release', '--bin', 'genvm-python',
			'--target=wasm32-wasi',
			'--features', if debug_sdk then 'sdk-debug' else '' end
		],
		cwd: cur_src
	)

	out = config.wasm_out_dir.join('genvm-python.wasm')

	py_targets = []

	py_targets << target_command(
		output_file: out,
		dependencies: [out_raw],
		command: [
			'cargo', 'run',
			out_raw, out
		],
		cwd: cur_src.parent.join('tools', 'softfloat-lib', 'patch-floats')
	)

	py_libs_file = config.wasm_out_dir.join('genvm-python-sdk.frozen')
	py_sdk_debug = target_command(
		output_file: py_libs_file,
		dependencies: py_deps + [cur_src.join('src', 'build_debug_sdk.rs')],
		commands: [
			['cargo', 'run', '--bin', 'genvm-python-build-debug-sdk', '--features', 'sdk-debug'],
			['cp', cur_src.join('target', 'sdk.frozen'), py_libs_file]
		]
	)

	if debug_sdk
		py_targets << py_sdk_debug
	end

	all.add_deps(
		target_alias(
			'all',
			config.wasm_out_dir.join('softfloat.wasm'),
			*py_targets
		)
	)
}
