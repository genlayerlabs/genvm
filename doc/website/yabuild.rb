POETRY_RUN = ['poetry', 'run', '-C', cur_src]

docs_build = cur_build.join('tree')
docs_out = root_build.join('out', 'docs')
docs_build.mkpath
docs_out.mkpath

LIB_SRC = root_src.join('runners', 'genlayer-py-std', 'src')

target_alias(
	"docs",
	target_command(
		commands: [
			['rm', '-rf', docs_build],
			['mkdir', '-p', docs_build.parent],
			['cp', '-r', cur_src.join('src'), docs_build],
			['cd', docs_build],
#			[RbConfig.ruby, cur_src.join('generate-other.rb'), LIB_SRC, docs_build.join('api', 'internal')],
			[*POETRY_RUN, 'sphinx-build', '-b', 'html', docs_build, docs_out],
		],
		cwd: cur_src,
		output_file: cur_build.join('docs.trg'), # always dirty
		dependencies: [],
	)
)
