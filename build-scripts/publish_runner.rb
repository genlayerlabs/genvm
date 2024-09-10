self.define_singleton_method(:target_publish_runner) do |name_base:, runner_dict:, out_dir:, files:, create_test_runner: config.createTestRunner|
	runner_json = cur_build.join("#{name_base}.runner.json")
	File.write(runner_json, JSON.dump(runner_dict))
	mark_as_config_generated runner_json
	fake_out = cur_build.join("#{name_base}.runner.trg")
	runner_publish_config = {
		create_test_runner: create_test_runner,
		fake_out: fake_out,
		out_dir: out_dir.join(name_base),
		files: files + [{
			path: "runner.json",
			read_from: runner_json,
		}]
	}
	publish_script = root_src.join('build-scripts', 'publish-runner.py')
	target_command(
		output_file: fake_out,
		commands: [
			[publish_script, JSON.dump(runner_publish_config)]
		],
		dependencies: [publish_script, runner_json] + files.map { |f| f[:read_from] }
	)
end
