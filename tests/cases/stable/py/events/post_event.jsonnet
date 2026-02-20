local simple_deploy = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([simple_deploy.run('${jsonnetDir}/post_event.py') {
    "message"+: {
        "datetime": "2025-07-11T00:00:00Z"
    }
}])}
