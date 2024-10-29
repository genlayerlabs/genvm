dev_container = find_target /\/cpython-dev-container$/

make_runner = Proc.new { |name, runner_name: name, extra_seq: []|
	base_lib_dir = cur_src.join(name)
	raise "#{base_lib_dir} does not exist" if not base_lib_dir.exist?
	lib_files = Dir.glob(base_lib_dir.to_s + "/**/*.py")
	compile_dir = cur_build.join(name)
	compile_dir.mkpath
	out_file = compile_dir.join('compiled.zip')

	build_pyc_s = target_command(
		commands: [
			['rm', '-rf', compile_dir],
			['mkdir', '-p', compile_dir.join('src')],
			['cp', '-r', base_lib_dir, compile_dir.join('src')],
			[
				RbConfig.ruby, root_src.join('build-scripts', 'docker-run-in.rb'),
				'--log', cur_build.join('compile-lib-log'),
				'--id-file', dev_container.meta.output_file,
				'--out-dir', compile_dir,
				'--entrypoint', '/scripts-py/compile.sh',
				'--',
				'/out',
				'/scripts-py/save-compiled.sh', '/out/', 'compiled.zip', '.',
			]
		],
		dependencies: [dev_container] + lib_files,
		cwd: cur_src,
		output_file: out_file,
		pool: 'console',
	)

	runner_target = target_publish_runner(
		name_base: "py-lib-#{runner_name}",
		out_dir: config.runners_dir,
		files: [{ include: out_file }],
		runner_dict: {
			Seq: extra_seq + [
				# FIXME
				{ MapFile: { to: "/py/libs/", file: "src/" }},
			],
		},
		dependencies: [],
		expected_hash: config.runners.py_libs.send(runner_name).hash,
		create_test_runner: false,
	)

	target_alias(
		runner_name,
		runner_target,
		tags: ['all', 'runner'],
		inherit_meta: ['expected_hash', 'runner_dep_id'],
	)
}

make_runner.('cloudpickle')
make_runner.('google', runner_name: 'protobuf')
make_runner.('onnx', runner_name: 'tiny_onnx_reader')
make_runner.('word_piece_tokenizer')
make_runner.('genlayermodelwrappers', extra_seq: [
	{ Depends: "py-lib-tiny_onnx_reader:#{config.runners.py_libs.tiny_onnx_reader.hash}" },
	{ Depends: "py-lib-protobuf:#{config.runners.py_libs.protobuf.hash}" },
	{ Depends: "py-lib-word_piece_tokenizer:#{config.runners.py_libs.word_piece_tokenizer.hash}" },
	{ Depends: "onnx-model-all-MiniLM-L6-v2:#{config.runners.onnx_models.all_MiniLM_L6_v2.hash}" },
])
