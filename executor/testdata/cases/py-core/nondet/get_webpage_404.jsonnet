local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/get_webpage_404.py') {
    "calldata": |||
        {
            "method": "main"
        }
    |||
}
