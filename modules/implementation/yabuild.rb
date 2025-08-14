executor_target = find_target 'genvm/executor/bin'

bin = target_cargo_build(
	name: "genvm-modules",
	target: config.executor_target,
	profile: config.profile,
	out_file: config.bin_dir.join('genvm-modules'),
	flags: executor_target.meta.cargo_flags,
	env: executor_target.meta.env,
) {
	order_only_inputs.push(*executor_target.order_only_inputs)
}

llm_config_target = target_copy(
	dest: config.out_dir.join('config', 'genvm-module-llm.yaml'),
	src: [cur_src.join('llm-default-config.yaml')],
)

web_config_target = target_copy(
	dest: config.out_dir.join('config', 'genvm-module-web.yaml'),
	src: [cur_src.join('web-default-config.yaml')],
)

lib_copy_targets = ['lib-genvm.lua', 'lib-web.lua', 'lib-llm.lua', 'inspect.lua', 'value2json.lua'].map { |ct|
	target_copy(
		dest: config.out_dir.join('share', 'lib', 'genvm', 'lua', ct),
		src: [cur_src.join('scripting', ct)],
	)
}

script_copy_targets = ['web-default.lua', 'llm-default.lua'].map { |ct|
	target_copy(
		dest: config.out_dir.join('scripts', ct),
		src: [cur_src.join('scripting', ct)],
	)
}

target_alias(
	'modules',
	bin,
	web_config_target, llm_config_target,
	*script_copy_targets, *lib_copy_targets,
	tags: ['all'],
)
