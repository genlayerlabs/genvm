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

run_codegen.(root_src.join('executor', 'codegen', 'data', 'result-codes.json'), cur_src.join('src', 'genlayer', 'std', '_internal', 'result_codes.py'))
