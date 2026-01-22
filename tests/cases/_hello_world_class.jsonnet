local simple_deploy_then_write = import 'templates/simple_deploy_then_write.jsonnet';
simple_deploy_then_write.run('${jsonnetDir}/${fileBaseName}.py', 'foo')
