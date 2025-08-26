local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/contract.py') {
    "calldata": |||
        {
            "method": "foo",
            "args": []
        }
    |||
}
