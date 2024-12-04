local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/multi_contract.py') {
    "calldata": |||
        {
            "method": "__init__",
            "args": []
        }
    |||,
    message+: {
        "is_init": true
    }
}
