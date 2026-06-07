local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
// Empty message-fee-allocation tree => the PostMessage emit has no matching node
// and must hard-trap (OOM fees internal). Overrides the runner default of three
// wildcard catch-all nodes (origin/fees.py DEFAULT_*_MESSAGE_ALLOC).
{entry: util.addPaths([simple_deploy.run('${jsonnetDir}/${fileBaseName}.py') {
	message_fee_allocation: [],
}])}
