local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/nondet.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
