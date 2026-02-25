local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/error_msg_overridden.py', '#error') {
	next: [super.next[0] {
		message+: {
			value: 100
		}
	}],
}])}
