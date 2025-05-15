include_dir 'build-scripts/ya-build-plugins'

config.out_dir.mkpath
config.bin_dir.mkpath

project('genvm') {
	include_dir 'runners'
	include_dir 'executor'
	include_dir 'doc/website'
}
