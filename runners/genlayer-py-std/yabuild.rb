run_codegen = Proc.new { |inp, out, tags: [], **kwargs, &blk|
	script = root_src.join('executor', 'codegen', 'templates', 'py.rb')
	target_command(
		output_file: out,
		command: [
			RbConfig.ruby, script, inp, out,
		],
		dependencies: [inp, script],
		tags: ['codegen'] + tags,
		**kwargs, &blk
	)
}
codegen = target_alias("codegen",
	run_codegen.(root_src.join('executor', 'codegen', 'data', 'builtin-prompt-templates.json'), cur_src.join('src', 'genlayer', 'std', '_internal', 'prompt_ids.py')),
	run_codegen.(root_src.join('executor', 'codegen', 'data', 'result-codes.json'), cur_src.join('src', 'genlayer', 'std', '_internal', 'result_codes.py')),
)

base_genlayer_lib_dir = cur_src.join('src')
lib_files = Dir.glob(base_genlayer_lib_dir.to_s + "/**/*.py")

compile_dir = cur_build.join('genlayer_compile_dir')
compile_dir.mkpath

runner_name = 'py-genlayer-std'

out_dir = config.out_dir.join('share', 'genvm', 'runners', runner_name)
expected_hash = 'test'

runner_json = {
	Seq: [
		{ MapFile: { to: "/py/libs/", file: "src/" }},
	],
}

py_genlayer_std_runner = target_command(
	commands: [
		['rm', '-rf', compile_dir],
		['mkdir', '-p', compile_dir],
		['cp', '-r', base_genlayer_lib_dir, compile_dir],

		$runner_precompile_command.(compile_dir),
		$runner_package_command.(
			'--expected-hash', expected_hash,
			'--src-dir', compile_dir,
			'--out-dir', out_dir,
			'--runner-json', JSON.dump(runner_json),
		)
	],
	dependencies: lib_files + [$runner_nix_target],
	output_file: out_dir.join("#{expected_hash}.tar"),
	tags: ['all', 'runner'],
) {
	meta.expected_hash = expected_hash
	meta.runner_id = "#{runner_name}:#{expected_hash}"
}

nix_target = find_target /\/runners\/nix$/

cum_name = 'py-genlayer'
cum_hash = 'test'
cum_out_dir = config.out_dir.join('share', 'genvm', 'runners', cum_name)
cum_runner_json = {
	Seq: [
		{ With: { runner: "<contract>", action: { MapFile: { file: "file", to: "/contract.py" } } } },
		{ SetArgs: ["py", "-u", "-c", "import contract; import genlayer.std._internal.runner"] },
		{ Depends: "py-lib-cloudpickle:#{config.runners.py_libs.cloudpickle.hash}" },
		{ Depends: py_genlayer_std_runner.meta.runner_id },
		{ Depends: nix_target.meta.cpython_id },
	],
}

target_command(
	output_file: cum_out_dir.join("#{cum_hash}.tar"),
	commands: [
		$runner_package_command.(
			'--expected-hash', cum_hash,
			'--src-dir', cur_src,
			'--out-dir', cum_out_dir,
			'--config', '#none',
			'--runner-json', JSON.dump(cum_runner_json),
		)
	],
	dependencies: [$runner_nix_target],
	tags: ['all', 'runner'],
)

cum_name = 'py-genlayer-multi'
cum_hash = 'test'
cum_out_dir = config.out_dir.join('share', 'genvm', 'runners', cum_name)
cum_runner_json = {
	Seq: [
		{ With: { runner: "<contract>", action: { MapFile: { file: "contract/", to: "/contract/" } } } },
		{ SetArgs: ["py", "-u", "-c", "import contract; import genlayer.std._internal.runner"] },
		{ Depends: "py-lib-cloudpickle:#{config.runners.py_libs.cloudpickle.hash}" },
		{ Depends: py_genlayer_std_runner.meta.runner_id },
		{ Depends: nix_target.meta.cpython_id },
	],
}

target_command(
	output_file: cum_out_dir.join("#{cum_hash}.tar"),
	commands: [
		$runner_package_command.(
			'--expected-hash', cum_hash,
			'--src-dir', cur_src,
			'--out-dir', cum_out_dir,
			'--config', '#none',
			'--runner-json', JSON.dump(cum_runner_json),
		)
	],
	dependencies: [$runner_nix_target],
	tags: ['all', 'runner'],
)

root_build.join('docs').mkpath
cur_build.join('docs').mkpath

POETRY_RUN = ['poetry', 'run', '-C', root_src.join('build-scripts', 'doctypes')]

docs_out = root_build.join('py-docs')
target_alias(
	"docs",
	target_command(
		commands: [
			['rm', '-rf', docs_out],
			['mkdir', '-p', docs_out.parent],
			['cp', '-r', root_src.join('build-scripts', 'doctypes', 'docs_base'), docs_out],
			['cd', docs_out],
			[RbConfig.ruby, root_src.join('build-scripts', 'doctypes', 'generate-other.rb'), cur_src.join('src'), docs_out.join('api', 'internal')],
			[*POETRY_RUN, 'sphinx-build', '-b', 'html', docs_out, docs_out.join('docs')],
			['zip', '-9', '-r', docs_out.parent.join('py-docs.zip'), 'docs']
		],
		cwd: cur_src,
		output_file: root_build.join('docs', 'py', 'docs.trg'),
		dependencies: [],
	)
)

types_out = root_build.join('py-types')
libs = root_src.join('runners', 'py-libs', 'pure-py').children.filter { |c| c.directory? }
target_alias(
	"types",
	target_command(
		commands: [
			['mkdir', '-p', types_out],
			['cd', types_out],
			['cp', '-r', cur_src.join('src', 'genlayer'), types_out],
			*libs.map { |l| ['cp', '-r', l, types_out] },
			['touch', types_out.join('google', '__init__.py')],
			*libs.map { |l| l.basename.to_s }.chain(['genlayer']).map { |name|
				[*POETRY_RUN, 'python3', '-m', 'pyright', '--createstub', name]
			},
			['zip', '-9', '-r', types_out.parent.join('py-types.zip'), 'typings'],
		],
		cwd: cur_src,
		output_file: types_out.join('trg'),
		dependencies: [],
		tags: ['types'],
	) {
		outputs.push types_out.parent.join('py-types.zip')
	}
)
