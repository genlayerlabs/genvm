local simple_deploy = import 'templates/simple_deploy.jsonnet';
simple_deploy.run('${jsonnetDir}/contract.zip') {
    "prepare": '${jsonnetDir}/prepare.py',
}
