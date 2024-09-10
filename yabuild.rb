eval_script('build-scripts/cargo.rb')
eval_script('build-scripts/publish_runner.rb')

config.out_dir.mkpath
config.bin_dir.mkpath

project('genvm') {
	include_dir 'tools/softfloat-lib'
	include_dir 'sdk-python'
	include_dir 'genvm'
}
