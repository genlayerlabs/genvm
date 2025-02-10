local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/balance_eth.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
    "balances": {
        "AQAAAAAAAAAAAAAAAAAAAAAAAAA=": 10,
    },
}
