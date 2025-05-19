include_dir 'genlayer-py-std'

latest_uids_file = config.runners_dir.join('latest.json')

latest_uids_file_tmp = cur_build.join('latest.json')

existing_runners_uids_file = cur_build.join('existing.json')
all_uids_file = config.runners_dir.join('all.json')

build_out = cur_build.join('nix-out')

deps = cur_src.glob('**/*.{nix,py,c,rb,json}')
deps.sort!

deps_registry = deps.filter { |x|
	x.relative_path_from(cur_src).to_s.start_with?('support/registry')
}

deps.filter! { |x|
	not x.relative_path_from(cur_src).to_s.start_with?('support/registry')
}

get_existing = target_command(
	output_file: existing_runners_uids_file,
	commands: [
		['bash', '-c', "nix eval --pure-eval --verbose --pure-eval --read-only --show-trace --json ./support/registry#registry > '#{existing_runners_uids_file}'"]
	],
	dependencies: deps_registry,
)

make_latest_json = target_command(
	output_file: latest_uids_file_tmp,
	command: [
		'bash', '-c', "nix eval --verbose --pure-eval --read-only --show-trace --json --file ./latest.nix > '#{latest_uids_file_tmp}'",
	],
	dependencies: deps,
)

build_and_make_latest = target_command(
	output_file: latest_uids_file,
	commands: [
		[
			'nix', 'build',
			'--file', './build-here.nix',
			'--show-trace',
			'--pure-eval',
			'--verbose',
			'-o', build_out
		],
		['echo', 'build done'],
		['mkdir', '-p', config.runners_dir],
		[
			'cp', '-r', '--no-preserve=timestamps,mode,ownership', build_out.to_s + '/.', config.runners_dir,
		],
		[
			'cp', latest_uids_file_tmp, latest_uids_file
		]
	],
	pool: 'console',
	dependencies: ['tags/codegen', make_latest_json] + deps,
)

make_all = target_command(
	output_file: all_uids_file,
	command: ['bash', '-c', "'#{config.tools.python3}' ./support/registry/tools merge-registries '#{existing_runners_uids_file}' '#{latest_uids_file}' > '#{all_uids_file}'"],
	dependencies: [
		latest_uids_file, existing_runners_uids_file,
	],
)

install_tool = target_command(
	output_file: config.out_dir.join('scripts', 'runners-registry', '__main__.py'),
	commands: [
		['cp', '-r', cur_src.join('support', 'registry', 'tools').to_s + '/.', config.out_dir.join('scripts', 'runners-registry')],
		['find', config.out_dir.join('scripts', 'runners-registry'), '-name', '*.pyc', '-delete'],
		['find', config.out_dir.join('scripts', 'runners-registry'), '-type', 'd', '-empty', '-delete'],
	],
	dependencies: deps_registry,
)

target_alias(
	'runners',
	build_and_make_latest,
	make_all,
	install_tool,
	tags: ['all'],
)
