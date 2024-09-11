project('genvm') {
	modules_dir = config.out_dir.join('lib', 'genvm-modules')

	modules = [
		target_cargo_build(
			name: "lib",
			profile: config.profile,
			out_file: modules_dir.join('libweb.so'),
			dir: cur_src.join('modules', 'default-impl', 'web-funcs')
		),
		target_cargo_build(
			name: "lib",
			profile: config.profile,
			out_file: modules_dir.join('libllm.so'),
			dir: cur_src.join('modules', 'default-impl', 'llm-funcs')
		)
	]

	mock = target_cargo_build(
		name: "genvm-mock",
		profile: config.profile,
		out_file: config.bin_dir.join('genvm-mock')
	)

	all.add_deps(
		target_alias('all', mock, *modules),
		target_copy(dest: config.out_dir.join('share', 'genvm', 'default-config.json'), src: cur_src.join('default-config.json'))
	)
}
