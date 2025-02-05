self.define_singleton_method(:target_publish_runner) do |name_base:, runner_dict:, out_dir:, files:, dependencies: [], expected_hash:, tags: []|
	raise "hash can nott be nil, use 'test'" if expected_hash.nil?
	runner_json = cur_build.join("#{name_base}.runner.json")
	File.write(runner_json, JSON.dump(runner_dict))
	mark_as_config_generated runner_json
	runner_publish_config = {
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
	new_json_data = JSON.dump(runner_publish_config)
	File.write(runner_publish_config_file, new_json_data)

	mark_as_config_generated runner_publish_config_file
	publish_script = root_src.join('build-scripts', 'publish-runner.py')

	out_file = out_dir.join(name_base, "#{expected_hash}.tar")

	target_command(
		output_file: out_file,
		commands: [
			[config.tools.python3, publish_script, runner_publish_config_file]
		],
		dependencies: [publish_script, runner_json] + dependencies + fdeps,
		tags: tags,
	) {
		meta.expected_hash = expected_hash
		meta.runner_dep_id = "#{name_base}:#{expected_hash}"
	}
end
