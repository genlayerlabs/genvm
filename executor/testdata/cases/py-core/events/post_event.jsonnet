local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/post_event.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
    "message"+: {
        "datetime": "2025-07-11T00:00:00Z"
    }
}
