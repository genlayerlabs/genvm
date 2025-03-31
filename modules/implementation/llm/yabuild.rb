executor_target = find_target 'genvm/executor/bin'

bin = target_alias(
	'llm',
	target_cargo_build(
		name: "genvm-module-llm",
		target: config.executor_target,
		profile: config.profile,
		out_file: config.bin_dir.join('genvm-module-llm'),
		flags: executor_target.meta.cargo_flags,
		env: executor_target.meta.env,
	),
	tags: ['all']
)

config_target = target_copy(
	dest: config.out_dir.join('etc', 'genvm-module-llm.yaml'),
	src: [cur_src.join('default-config.yaml')],
	tags: ['all'],
)
