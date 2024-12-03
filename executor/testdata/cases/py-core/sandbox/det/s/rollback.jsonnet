local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../code.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["gl.rollback_immediate('RB')"]
        }
    |||
}
