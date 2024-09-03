eval_script('build-scripts/cargo.rb')

config.out_dir = root_build.join('out')
config.out_dir.mkpath
config.wasm_out_dir = config.out_dir.join('wasm')
config.wasm_out_dir.mkpath
config.bin_dir = config.out_dir.join('bin')
config.bin_dir.mkpath

project('genvm') {
	include_dir 'tools/softfloat-lib'
	include_dir 'sdk-python'
	include_dir 'genvm'
}
