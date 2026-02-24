local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{
	entry: util.addPaths([
		simple_deploy.run('${jsonnetDir}/eq_prompt_comparative.py') { stable_hash: false }
	]),
}
