compile_cpython_ext = target_cargo_build(
	name: 'staticlib',
	dir: cur_src,
	profile: "release",
	target: "wasm32-wasip1",
)

check_sum = target_command(
	commands: [
		['tree', cur_src.join('target', 'wasm32-wasip1', 'release')],
		['tree', root_src.join('sdk-rust')],
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
