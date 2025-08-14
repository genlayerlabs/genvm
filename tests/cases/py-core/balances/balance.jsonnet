local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/${fileBaseName}.py') {
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
