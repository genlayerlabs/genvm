local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/to_str.py') {
    "calldata": |||
        {
            "method": "main"
        }
    |||
}
