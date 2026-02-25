local simple = import 'templates/simple_deploy_then_write.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/sandbox.py', 'main', ["gl.nondet.web.render('https://test-server.genlayer.com/static/genvm/hello.html', mode='text')"]) {
	next: [super.next[0] {
		stable_hash: false,
	}],
}])}
