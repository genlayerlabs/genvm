sdk_rust = find_target /genvm\/sdk-rust$/

compile_cpython_ext = target_cargo_build(
	name: 'staticlib',
	dir: cur_src,
	profile: "release",
	target: "wasm32-wasip1",
) {
	add_deps sdk_rust
}

dirs = [root_src.join('sdk-rust'), cur_src.join('target', 'wasm32-wasip1', 'release')]
dirs.map! { |d| d.relative_path_from(cur_src) }.sort!

check_sum = target_command(
	commands: [
		[root_src.join('build-scripts', 'dbg.sh'), *dirs],
		['cat', 'target/wasm32-wasip1/release/.fingerprint/cfg-if-dac41bbddfe2cdc5/lib-cfg_if.json'],
		['cat', 'target/wasm32-wasip1/release/.fingerprint/libc-1fb03017ae73ebbb/lib-libc.json'],
		['cat', 'target/wasm32-wasip1/release/.fingerprint/memoffset-8f8cbf46c23111d8/lib-memoffset.json'],
		['echo', ''],
		['sha256sum', '-c', cur_src.join('lib.sha256')],
	],
	tags: ['checksum'],
	output_file: cur_build.join('check-sha256.dirty'),
	dependencies: [compile_cpython_ext],
)

compile_cpython_ext = target_alias(
	'extension', compile_cpython_ext, check_sum
) {
	meta.output_file = compile_cpython_ext.output_file
}
