include_dir 'genlayer-py-std'

latest_uids_file = config.runners_dir.join('latest.json')

build_out = cur_build.join('nix-out')

deps = cur_src.glob('**/*.{nix,py,c,rb,json}')
deps.sort!

target_alias(
	'runners',
	target_command(
		output_file: latest_uids_file,
		commands: [
			['bash', '-c', 'nix eval --read-only --show-trace --json --file ./latest.nix > "' + "#{latest_uids_file}" + '"'],
			[
				'nix', 'build',
				'--file', './build-here.nix',
				'--show-trace',
				'--pure-eval',
				'-o', build_out
			],
			['mkdir', '-p', config.runners_dir],
			[
				'cp', '-r', '--no-preserve=timestamps,mode,ownership', build_out.to_s + '/.', config.runners_dir,
			],
		],
		pool: 'console',
		dependencies: ['tags/codegen'] + deps,
	),
	tags: ['all']
)
