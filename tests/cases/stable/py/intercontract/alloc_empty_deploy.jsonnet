local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
// Empty allocation tree => the DeployContract emit (matched by Address::zero() +
// CallKey::DEPLOY) has no node and must hard-trap (OOM fees internal).
{entry: util.addPaths([simple_deploy.run('${jsonnetDir}/${fileBaseName}.py') {
	message_fee_allocation: [],
}])}
