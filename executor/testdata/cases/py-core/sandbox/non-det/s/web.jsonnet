local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../code.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["gl.get_webpage('http://genvm-test/hello.html', mode='text')"]
        }
    |||
}
