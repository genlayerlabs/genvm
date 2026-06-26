local simple = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple.run('${jsonnetDir}/custom_malformed.py') {stable_hash: false}])}
