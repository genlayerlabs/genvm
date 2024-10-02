eval_script('build-scripts/cargo.rb')
eval_script('build-scripts/publish_runner.rb')

config.out_dir.mkpath
config.bin_dir.mkpath

project('genvm') {
	include_dir 'sdk-rust'
	include_dir 'runners'
	include_dir 'genvm'
}
