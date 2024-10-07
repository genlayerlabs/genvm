local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/contract.py') {
    "calldata": |||
        {
            "method": "__init__",
            "args": []
        }
    |||
}
