local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../code.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["gl.nondet.web.render('http://genvm-test/hello.html', mode='text')"]
        }
    |||
}
