local simple_deploy_then_write = import 'templates/simple_deploy_then_write.jsonnet';
local r = simple_deploy_then_write.run('${jsonnetDir}/${fileBaseName}.py', 'bench');
[
    r[0],
    r[1] + {benchmark: true},
]
