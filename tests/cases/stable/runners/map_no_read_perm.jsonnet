local simple = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/map_no_read_perm.py') {stable_hash: false, permissions: 'wscn'}])}
