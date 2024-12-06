local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/store_proxy.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
