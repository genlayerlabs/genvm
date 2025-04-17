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
)

config_target = target_copy(
	dest: config.out_dir.join('config', 'genvm-module-llm.yaml'),
	src: [cur_src.join('default-config.yaml')],
)

lua_lib_target = target_copy(
	dest: config.out_dir.join('share', 'lib', 'genvm', 'greyboxing', 'lib-greyboxing.lua'),
	src: [cur_src.join('scripting/lib-greyboxing.lua')],
)

script_target = target_copy(
	dest: config.out_dir.join('scripts', 'genvm-greyboxing.lua'),
	src: [cur_src.join('scripting/greyboxing.lua')],
)

find_target('genvm/modules/all').inputs.push(bin, config_target, script_target, lua_lib_target)
