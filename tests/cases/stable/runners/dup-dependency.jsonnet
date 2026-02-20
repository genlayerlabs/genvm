local simple = import 'templates/simple.jsonnet';
local util = import 'templates/util.jsonnet';
{
	prepare: '${jsonnetDir}/dup-dependency-prepare.py',
	entry: util.addPaths([simple.run('${jsonnetDir}/dup-dependency.py')])
}
