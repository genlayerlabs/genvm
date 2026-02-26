local simple = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/alloc_generic.py')])}
