local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/vecdb.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
