executor_target = find_target 'genvm/executor/bin'

bin = target_alias(
	'web',
	target_cargo_build(
		name: "genvm-module-web",
		target: config.executor_target,
		profile: config.profile,
		out_file: config.bin_dir.join('genvm-module-web'),
		flags: executor_target.meta.cargo_flags,
		env: executor_target.meta.env,
	) {
		order_only_inputs.push(*executor_target.meta.order_only_inputs)
	},
	tags: ['all']
)

config_target = target_copy(
	dest: config.out_dir.join('config', 'genvm-module-web.yaml'),
	src: [cur_src.join('default-config.yaml')],
	tags: ['all'],
)

find_target('genvm/modules/all').inputs.push(bin, config_target)
