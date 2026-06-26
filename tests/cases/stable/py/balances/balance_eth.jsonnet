local deploy_then = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
local bal = {
	"balances": {
		"AQAAAAAAAAAAAAAAAAAAAAAAAAA=": 10,
	},
};
local base = deploy_then.run('${jsonnetDir}/balance_eth.py', 'main');
{entry: util.addPaths([base + bal + {next: [base.next[0] + bal]}])}
