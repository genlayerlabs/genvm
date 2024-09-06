project('genvm') {
	modules_dir = config.out_dir.join('lib', 'genvm-modules')

	modules = [
		target_cargo_build(
			name: "lib",
			profile: config.profile,
			out_file: modules_dir.join('libnondet-funcs.so'),
			dir: cur_src.join('modules', 'default-impl', 'nondet-funcs')
		)
	]

	mock = target_cargo_build(
		name: "genvm-mock",
		profile: config.profile,
		out_file: config.bin_dir.join('genvm-mock')
	)

	all.add_deps(
		target_alias('all', mock, *modules)
	)
}
