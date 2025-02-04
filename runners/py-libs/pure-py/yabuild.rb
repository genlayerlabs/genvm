make_runner = Proc.new { |name, runner_name: name, extra_seq: []|
	base_lib_dir = cur_src.join(name)
	raise "#{base_lib_dir} does not exist" if not base_lib_dir.exist?

	lib_files = Dir.glob(base_lib_dir.to_s + "/**/*.py")

	compile_dir = cur_build.join(name)
	compile_dir.mkpath

	out_dir = config.out_dir.join('share', 'genvm', 'runners', "py-lib-#{runner_name}")
	expected_hash = config.runners.py_libs.send(runner_name).hash

	runner_json = {
		Seq: extra_seq + [
			{ MapFile: { to: "/py/libs/", file: "src/" }},
		],
	}

	target_command(
		commands: [
			['rm', '-rf', compile_dir],
			['mkdir', '-p', compile_dir.join('src')],
			['cp', '-r', base_lib_dir, compile_dir.join('src')],

			$runner_precompile_command.(compile_dir),
			$runner_package_command.(
				'--expected-hash', expected_hash,
				'--src-dir', compile_dir,
				'--out-dir', out_dir,
				'--runner-json', JSON.dump(runner_json),
			)
		],
		dependencies: lib_files + [$runner_nix_target],
		output_file: out_dir.join("#{expected_hash}.tar"),
		tags: ['all', 'runner'],
	) {
		meta.expected_hash = expected_hash
		meta.runner_id = "py-lib-#{runner_name}:#{expected_hash}"
	}
}

cloudpickle = make_runner.('cloudpickle')
protobuf = make_runner.('google', runner_name: 'protobuf')
tiny_onnx = make_runner.('onnx', runner_name: 'tiny_onnx_reader')
word_piece_tokenizer = make_runner.('word_piece_tokenizer')

make_runner.(
	'genlayermodelwrappers',
	extra_seq: [cloudpickle, protobuf, tiny_onnx, word_piece_tokenizer].map { |v|
		{ Depends: v.meta.runner_id }
	} + [{ Depends: "onnx-model-all-MiniLM-L6-v2:#{config.runners.onnx_models.all_MiniLM_L6_v2.hash}" }],
)
