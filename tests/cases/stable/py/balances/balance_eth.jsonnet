local simple_deploy = import 'templates/simple_deploy.jsonnet';
simple_deploy.run('${jsonnetDir}/balance_eth.py') {
    "balances": {
        "AQAAAAAAAAAAAAAAAAAAAAAAAAA=": 10,
    },
}
