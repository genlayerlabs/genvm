self.define_singleton_method(:target_publish_runner) do |name_base:, runner_dict:, out_dir:, files:, create_test_runner: config.createTestRunner, dependencies: [], expected_hash: nil|
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
		}],
		expected_hash: expected_hash,
	}
	fdeps = files.map { |f|
		if f.has_key? :include
			f[:include]
		else
			f[:read_from]
		end
	}
	runner_publish_config_file = cur_build.join("#{name_base}.publish-config.json")
	File.write(runner_publish_config_file, JSON.dump(runner_publish_config))
	mark_as_config_generated runner_publish_config_file
	publish_script = root_src.join('build-scripts', 'publish-runner.py')
	if expected_hash.nil?
		out_file = fake_out
	else
		out_file = out_dir.join(name_base, "#{expected_hash}.zip")
	end
	target_command(
		output_file: out_file,
		commands: [
			[config.python, publish_script, runner_publish_config_file]
		],
		dependencies: [publish_script, runner_json] + dependencies + fdeps
	) {
		meta.expected_hash = expected_hash
	}
end
