local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../code.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["gl.advanced.rollback_immediate('RB')"]
        }
    |||
}
