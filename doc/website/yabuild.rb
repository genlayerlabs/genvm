POETRY_RUN = ['poetry', 'run', '-C', cur_src]

docs_out = root_build.join('out', 'docs')
docs_out.mkpath

docs_out_txt = cur_build.join('txt')
docs_out_txt.mkpath

LIB_SRC = root_src.join('runners', 'genlayer-py-std', 'src')

codegen_src =  cur_src.parent.parent.join('executor', 'codegen')

target_alias(
	"docs",
	target_command(
		commands: [
			[RbConfig.ruby, codegen_src.join('templates', 'rst.rb'), codegen_src.join('data', 'public-abi.json'), cur_src.join('src', 'spec', 'appendix', 'constants.rst')],

			['rm', '-rf', docs_out_txt],
			['mkdir', '-p', docs_out_txt],

			[config.tools.python3, root_src.join('runners', 'support', 'match-tags.py'), cur_src.join('src', 'impl-spec', 'appendix', 'runners-versions.json')],

			[*POETRY_RUN, 'sphinx-build', '-b', 'html', cur_src.join('src'), docs_out],
			[*POETRY_RUN, 'sphinx-build', '-b', 'text', cur_src.join('src'), docs_out_txt],
			[RbConfig.ruby, cur_src.join('merge-txts.rb'), docs_out_txt.join('api'), docs_out.join('_static', 'ai', 'api.txt')],
			[RbConfig.ruby, cur_src.join('merge-txts.rb'), docs_out_txt.join('spec'), docs_out.join('_static', 'ai', 'spec.txt')],
			[RbConfig.ruby, cur_src.join('merge-txts.rb'), docs_out_txt.join('impl-spec'), docs_out.join('_static', 'ai', 'impl-spec.txt')],
		],
		cwd: root_src,
		output_file: cur_build.join('docs.trg'), # always dirty
		dependencies: [],
	)
)
