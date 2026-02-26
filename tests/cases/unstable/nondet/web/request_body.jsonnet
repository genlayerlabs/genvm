local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple_deploy.run('${jsonnetDir}/request_body.py') { stable_hash: false }])}
