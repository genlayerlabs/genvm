local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/balance.py') {
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
