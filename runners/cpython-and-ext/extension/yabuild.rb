compile_cpython_ext = target_cargo_build(
	name: 'staticlib',
	dir: cur_src,
	profile: "release",
	target: "wasm32-wasip1",
)

compile_cpython_ext = target_alias(
	'extension', compile_cpython_ext
) {
	meta.output_file = compile_cpython_ext.output_file
}

target_command(
	command: ['sha256sum', '-c', cur_src.join('lib.sha256')],
	tags: ['checksum'],
	output_file: cur_build.join('check-sha256.dirty'),
	dependencies: [compile_cpython_ext],
)
