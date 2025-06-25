local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../code.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["gl.nondet.web.render('https://test-server.genlayer.com/static/genvm/hello.html', mode='text')"]
        }
    |||
}
