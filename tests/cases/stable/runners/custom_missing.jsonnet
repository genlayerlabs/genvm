local simple = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/custom_missing.py') {stable_hash: false}])}
