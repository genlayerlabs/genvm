sdk_rust = find_target /genvm\/sdk-rust$/

docker_id_file = cur_build.join('docker-id.txt')

files = [cur_src.join('Dockerfile'), cur_src.join('Cargo.toml'), cur_src.join('Cargo.lock'), cur_src.join('docker-build.sh')] +  Dir::glob(cur_src.join('src').to_s + '/**/*') + Dir::glob(cur_src.join('.cargo').to_s + '/**/*')

docker_build_dev_container = target_command(
	command: [
		RbConfig.ruby, root_src.join('build-scripts', 'docker-build.rb'),
		cur_build.join('build-log'),
		docker_id_file,
		root_src.relative_path_from(cur_src),
		'Dockerfile',
	],
	dependencies: files + [sdk_rust, root_src.join('build-scripts', 'docker-build.rb')],
	cwd: cur_src,
	output_file: docker_id_file,
	pool: 'console',
)

out_dir = cur_build.join('out')
# output_raw_file = out_dir.join('_wasi.raw.so')
output_file = out_dir.join('libgenvm_cpython_ext.a')

docker_run_script = root_src.join('build-scripts', 'docker-run-in.rb')

renamer = find_target /\/rename-wasm-module$/

compile_cpython_ext = target_command(
	commands: [
		[
			RbConfig.ruby, docker_run_script,
			'--log', cur_build.join('run-log'),
			'--id-file', docker_id_file,
			'--out-dir', out_dir,
			'--network', 'none',
			'--entrypoint', '/opt/genvm/runners/cpython-and-ext/extension/docker-build.sh'
		],
		# [renamer.meta.output_file, output_raw_file, output_file, '_wasi.so'],
	],
	dependencies: [docker_build_dev_container, docker_run_script, renamer],
	cwd: cur_src,
	output_file: output_file,
	pool: 'console',
)

compile_cpython_ext = target_alias(
	'cpython-extension-lib', compile_cpython_ext
) {
	meta.output_file = compile_cpython_ext.output_file
}
