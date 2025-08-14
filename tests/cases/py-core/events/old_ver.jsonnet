local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/old_ver.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
