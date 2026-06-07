local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
// A matching wildcard internal-finalized node, but budget=1 (< the computed
// message_fee, which is ~34836 for foo(1,2) on finalized). The find_map matches,
// then consume_message_fee_internal must trap on `fee_cost > node.budget`.
// Mirrors origin/fees.py DEFAULT_INTERNAL_FIN_MESSAGE_ALLOC with budget lowered.
{entry: util.addPaths([simple_deploy.run('${jsonnetDir}/${fileBaseName}.py') {
	message_fee_allocation: [
		{
			budget: 1,
			recipient: null,
			call_key: null,
			on: 'finalized',
			fee_params: {
				Internal: {
					execution_budget_per_round: 1024,
					rotations: [4, 4, 4, 4, 4],
					leader_timeunits_allocation: 5,
					validator_timeunits_allocation: 5,
				},
			},
			children: [],
		},
	],
}])}
