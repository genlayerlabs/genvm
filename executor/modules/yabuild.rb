project('modules') {
	target_alias('all', tags: ['all'])
	include_dir('implementation/llm')
	include_dir('implementation/web')
}
