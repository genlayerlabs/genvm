local simple = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/register_no_perm.py') {stable_hash: false}])}
