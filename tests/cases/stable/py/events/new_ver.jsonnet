local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/new_ver.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
