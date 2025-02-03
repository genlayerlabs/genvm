project('models') {
	['all-MiniLM-L6-v2'].each { |name|
		deps = cur_src.join(name).glob('**/*')
		hash = config.runners.onnx_models.send(name.gsub(/-/, '_')).hash
		out = config.out_dir.join('share', 'genvm', 'runners', "onnx-model-#{name}")
		target_command(
			output_file: out.join("#{hash}.tar"),
			command: $runner_package_command.('--expected-hash', hash, '--src-dir', name, '--out-dir', out),
			tags: ['all', 'runner'],
			dependencies: deps,
		)
	}
}
