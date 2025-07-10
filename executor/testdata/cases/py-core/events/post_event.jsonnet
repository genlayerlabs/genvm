local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/post_event.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
