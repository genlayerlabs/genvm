local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../code.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["exec(\"json.loads.__name__ = 'haha'\")"]
        }
    |||
}
