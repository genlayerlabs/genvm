local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/contract.zip') {
    "calldata": |||
        {
            "method": "__init__",
            "args": []
        }
    |||
}
