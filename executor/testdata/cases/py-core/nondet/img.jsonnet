local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/img.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
