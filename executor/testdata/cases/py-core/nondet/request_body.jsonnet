local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/request_body.py') {
    "calldata": |||
        {
            "method": "main",
        }
    |||
}
