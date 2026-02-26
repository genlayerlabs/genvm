local msg = import 'templates/message.json';
local util = import 'templates/util.jsonnet';
local simple_deploy = import 'templates/simple_deploy.jsonnet';

{
	tags: ['fuzz'],
	entry: util.addPaths([
		simple_deploy.run('${jsonnetDir}/contract.py') {
			expected_semantics_components: [],
			modes: 'lvs',
			stable_hash: true,
		},
	]),
}
