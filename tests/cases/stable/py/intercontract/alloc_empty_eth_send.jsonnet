local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
// Empty allocation tree => the external EthSend has no matching node and must
// hard-trap (OOM fees external).
{entry: util.addPaths([simple_deploy.run('${jsonnetDir}/${fileBaseName}.py') {
	message_fee_allocation: [],
}])}
