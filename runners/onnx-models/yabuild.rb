add_model = Proc.new { |name|
	runner_target = target_publish_runner(
		name_base: "onnx-model-#{name}",
		out_dir: config.runners_dir,
		files: [{ path: 'model.onnx', read_from: cur_src.join(name + '.onnx') },],
		runner_dict: {
			Seq: [
				{ MapFile: { to: "/models/onnx/#{name}.onnx", file: "model.onnx" }},
			],
		},
		dependencies: [],
		expected_hash: config.runners.onnx_models.send(name.gsub(/-/, '_')).hash,
		tags: ['all', 'runner']
	)
}

project('models') {
	add_model.('all-MiniLM-L6-v2')
}
