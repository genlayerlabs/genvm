config.out_dir = root_build.join('out')
config.out_dir.mkpath

project('genvm') {
	include_dir 'tools/softfloat-lib'
	include_dir 'sdk-python'
	#include_dir 'genvm'
}
