local simple = import 'templates/simple_deploy_then_write.jsonnet';
local s = simple.run('${jsonnetDir}/rollback_imm.py', 'main');
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([
	s {
		next: [
			super.next[0] {
				modes: 'v',
				leader_nondet: [
					{
						"kind": "user_error",
						"value": "rollback"
					}
				]
			},
			super.next[0] {
				modes: 'v',
				leader_nondet: [
					{
						"kind": "user_error",
						"value": "other rollback"
					}
				]
			},
			super.next[0] {
				modes: 'v',
				leader_nondet: [
					{
						"kind": "user_error",
						"value": 1
					}
				]
			},
		],
	},
])}
