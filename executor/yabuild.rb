project('executor') {
	modules_dir = config.out_dir.join('lib', 'genvm-modules')

	base_env = {}
	compiler = config.tools.clang || config.tools.gcc
	linker = config.tools.mold || config.tools.lld
	# not config.tools.clang.nil? and
	if not compiler.nil? and not linker.nil?
		base_env['RUSTFLAGS'] ||= ''
		base_env['RUSTFLAGS'] << "-Clinker=#{Shellwords.escape compiler} -Clink-arg=-fuse-ld=#{Shellwords.escape linker}"
	end

	if config.executor.coverage
		base_env['RUSTFLAGS'] ||= ''
		base_env['RUSTFLAGS'] << " -Cinstrument-coverage"
	end

	modules = target_alias('modules',
		target_cargo_build(
			name: "dylib",
			profile: config.profile,
			out_file: modules_dir.join('libweb.so'),
			dir: cur_src.join('modules', 'default-impl', 'web-funcs'),
			flags: ['-Zprofile-rustflags'],
			env: base_env,
		),
		target_cargo_build(
			name: "dylib",
			profile: config.profile,
			out_file: modules_dir.join('libllm.so'),
			dir: cur_src.join('modules', 'default-impl', 'llm-funcs'),
			flags: ['-Zprofile-rustflags'],
			env: base_env,
		)
	)

	codegen = target_command(
		output_file: cur_src.join('src', 'host', 'host_fns.rs'),
		command: [
			RbConfig.ruby, cur_src.join('codegen', 'templates', 'host-rs.rb')
		],
		dependencies: [cur_src.join('codegen', 'data', 'host-fns.json')],
		tags: ['codegen'],
	)

	bin = target_alias(
		'bin',
		target_cargo_build(
			name: "genvm",
			profile: config.profile,
			out_file: config.bin_dir.join('genvm'),
			flags: ['-Zprofile-rustflags'],
			env: base_env,
		) {
			add_deps codegen
		}
	)

	config_target = target_copy(
		dest: config.out_dir.join('share', 'genvm', 'default-config.json'),
		src: cur_src.join('default-config.json'),
	)

	genvm_all = target_alias('all', bin, modules, config_target, tags: ['all'])

	target_command(
		output_file: cur_src.join('testdata', 'runner', 'host_fns.py'),
		command: [
			RbConfig.ruby, cur_src.join('codegen', 'templates', 'host-py.rb')
		],
		dependencies: [cur_src.join('codegen', 'data', 'host-fns.json')],
		tags: ['testdata']
	)

	if config.profile == "debug"
		target_c(
			output_file: root_build.join('fake-dlclose.so'),
			mode: "compile",
			file: cur_src.join('testdata', 'fake-dlclose.c'),
			cc: config.cc,
			flags: ['-g', '-pie', '-shared'],
			tags: ['testdata']
		)
	end
}
