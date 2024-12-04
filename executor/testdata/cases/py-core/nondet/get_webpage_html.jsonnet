local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/get_webpage.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["html"]
        }
    |||
}
