project('softfloat') {
	c_files_src = cur_src.join('berkeley-softfloat-cfiles.txt')
	reconfigure_on_change c_files_src
	c_files = File.read(c_files_src).lines.map { |l| l.strip }

	clang = Pathname.new(config.wasiSdk).join('bin', 'clang')
	sysroot = Pathname.new(config.wasiSdk).join('share', 'wasi-sysroot')

	c_file_targets = c_files.map.with_index { |cf, i|
		cf = cur_src.join(cf)
		cf_rel = cf.relative_path_from(cur_src)
		target_c_compile(
			output_file: cur_build.join(cf_rel.sub_ext('.o')),
			file: cf,
			cc: clang,
			flags: [
				'--target=wasm32-wasi', "--sysroot=#{sysroot}",
				'-flto', '-O3',
				'-DINLINE_LEVEL=9', '-DSOFTFLOAT_FAST_INT64',
				'-no-canonical-prefixes',
				'-Wno-builtin-macro-redefined', '-D__TIME__=0:42:42', '-D__DATE__=Jan 24 2024',
				"-frandom-seed=#{i}",
				"-I#{cur_src.join('spec').relative_path_from(root_build)}",
				"-I#{cur_src.join('berkeley-softfloat-3/source/include').relative_path_from(root_build)}"
			]
		)
	}

	raw = target_c_link(
		output_file: cur_build.join('softfloat.raw.wasm'),
		objs: c_file_targets,
		cc: clang,
		flags: [
			'--target=wasm32-wasi', "--sysroot=#{sysroot}",
			'-flto', '-O3',
			'-frandom-seed=0',
			'-Wl,--no-entry,--export-dynamic',
			'-static',
			'-lc'
		]
	)

	lib_patcher_build = target_cargo_build(
		name: 'genvm-softfloat-lib-patcher',
		dir: cur_src.join('patch-lib'),
	)

	target_alias(
		'rename-wasm-module',
		lib_patcher_build,
	) {
		meta.output_file = lib_patcher_build.output_file
	}

	out = cur_build.join('softfloat.wasm')
	softfloat_lib = target_alias(
		"lib",
		target_command(
			output_file: out,
			dependencies: [raw, lib_patcher_build],
			command: [
				lib_patcher_build.output_file,
				raw.output_file,
				out,
				"softfloat"
			],
			cwd: cur_src.join('patch-lib')
		)
	) {
		meta.output_file = out
	}

	runner_target = target_publish_runner(
		name_base: 'softfloat',
		out_dir: config.runners_dir,
		files: [
			{ path: 'softfloat.wasm', read_from: softfloat_lib.meta.output_file },
		],
		runner_dict: {
			LinkWasm: "softfloat.wasm",
		},
		expected_hash: config.runners.softfloat.hash,
	)

	target_alias(
		'runner',
		runner_target,
		tags: ['all', 'runner'],
		inherit_meta: ['expected_hash', 'runner_dep_id']
	)


	build_softfloat_patcher = target_cargo_build(
		name: 'genvm-softfloat-patcher',
		dir: cur_src.join('patch-floats')
	)
	build_softfloat_patcher = target_alias(
		'patcher',
		build_softfloat_patcher
	) {
		meta.output_file = build_softfloat_patcher.output_file
	}
}
