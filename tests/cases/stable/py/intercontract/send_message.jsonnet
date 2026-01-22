local simple_deploy = import 'templates/simple_deploy.jsonnet';
simple_deploy.run('${jsonnetDir}/send_message.py')
