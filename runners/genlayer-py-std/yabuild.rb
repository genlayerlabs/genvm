dev_container = find_target /runners\/cpython-dev-container$/

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
	run_codegen.(root_src.join('executor', 'codegen', 'data', 'builtin-prompt-templates.json'), cur_src.join('src', 'genlayer', 'std', 'prompt_ids.py')),
	run_codegen.(root_src.join('executor', 'codegen', 'data', 'result-codes.json'), cur_src.join('src', 'genlayer', 'std', 'result_codes.py')),
)

base_genlayer_lib_dir = cur_src.join('src')
lib_files = Dir.glob(base_genlayer_lib_dir.to_s + "/**/*.py")

compile_dir = cur_build.join('genlayer_compile_dir')
compile_dir.mkpath

out_file = compile_dir.join('compiled.zip')

build_pyc_s = target_command(
	commands: [
		['rm', '-rf', compile_dir],
		['mkdir', '-p', compile_dir],
		['cp', '-r', base_genlayer_lib_dir, compile_dir],
		[
			RbConfig.ruby, root_src.join('build-scripts', 'docker-run-in.rb'),
			'--log', cur_build.join('compile-lib-log'),
			'--id-file', dev_container.meta.output_file,
			'--out-dir', compile_dir,
			'--entrypoint', '/scripts-py/compile.sh',
			'--',
			'/out',
			'/scripts-py/save-compiled.sh', '/out/', 'compiled.zip', 'src',
		]
	],
	dependencies: [dev_container, codegen] + lib_files,
	cwd: cur_src,
	output_file: out_file,
	pool: 'console',
)

cpython_runner = find_target /runners\/cpython$/
cloudpickle_runner = find_target /runners\/py-libs\/cloudpickle$/

# extension_target = find_target /runners\/cpython-extension-lib$/

runner_target = target_publish_runner(
	name_base: 'py-genlayer-std',
	out_dir: config.runners_dir,
	files: [{ include: out_file }],
	runner_dict: {
		Seq: [
			{ MapFile: { to: "/py/libs/", file: "src/" }},
		],
	},
	dependencies: [build_pyc_s],
	expected_hash: 'test',
)

all_runner_target = target_publish_runner(
	name_base: 'py-genlayer',
	out_dir: config.runners_dir,
	files: [],
	runner_dict: {
		Seq: [
			{ MapCode: { to: "/contract.py" } },
			{ SetArgs: ["py", "-u", "-c", "import contract;import genlayer.std.runner as r;r.run(contract)"] },
			{ Depends: cloudpickle_runner.meta.runner_dep_id },
			{ Depends: runner_target.meta.runner_dep_id },
			{ Depends: cpython_runner.meta.runner_dep_id },
		],
	},
	dependencies: [runner_target],
	expected_hash: 'test',
)

target_alias(
	'py-genlayer',
	all_runner_target,
	tags: ['all', 'runner'],
	inherit_meta: ['expected_hash'],
)

root_build.join('docs').mkpath
cur_build.join('docs').mkpath

POETRY_RUN = 'poetry', 'run', '-C', cur_src

docs_out = root_build.join('py-docs')
docs_out.mkpath
target_alias(
	"docs",
	target_command(
		commands: [
			['cp', cur_src.join('docs_base', 'conf.py'), docs_out],
			[*POETRY_RUN, 'sphinx-apidoc', '-F', '-o', docs_out, cur_src.join('src')],
			[*POETRY_RUN, 'sphinx-build', '-b', 'html', docs_out, docs_out.join('docs')],
			['zip', '-9', '-r', docs_out.parent.join('py-docs.zip'), 'docs']
		],
		cwd: docs_out,
		output_file: root_build.join('docs', 'py', 'docs.trg'),
		dependencies: [],
	)
)

types_out = root_build.join('py-types')
types_out.mkpath
libs = root_src.join('runners', 'py-libs', 'pure-py').children.filter { |c| c.directory? }
target_alias(
	"types",
	target_command(
		commands: [
			['cp', '-r', cur_src.join('src', 'genlayer'), types_out],
			*libs.map { |l| ['cp', '-r', l, types_out] },
			['touch', types_out.join('google', '__init__.py')],
			*libs.map { |l| l.basename.to_s }.chain(['genlayer']).map { |name|
				[*POETRY_RUN, 'python3', '-m', 'pyright', '--createstub', name]
			},
			['zip', '-9', '-r', types_out.parent.join('py-types.zip'), 'typings'],
		],
		cwd: types_out,
		output_file: types_out.join('trg'),
		dependencies: [],
		tags: ['types'],
	) {
		outputs.push types_out.parent.join('py-types.zip')
	}
)
