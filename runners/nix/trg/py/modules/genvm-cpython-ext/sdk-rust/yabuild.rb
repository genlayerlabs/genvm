build_generator = target_cargo_build(
	name: 'witx-bindgen',
	dir: cur_src.join('third-party', 'wasi-rs', 'crates', 'witx-bindgen'),
	target: RUST_HOST_TARGET,
)

witx_file = root_src.join('executor', 'src', 'wasi', 'witx', 'genlayer_sdk.witx')

output_file = cur_src.join('src', 'generated.rs')

target_alias(
	'sdk-rust',
	target_command(
		commands: [
			[build_generator.output_file, witx_file, output_file],
			['cargo', 'fmt']
		],
		dependencies: [build_generator, witx_file, root_src.join('executor', 'src', 'wasi', 'witx', 'genlayer_sdk_types.witx')],
		output_file: output_file,
	)
)
