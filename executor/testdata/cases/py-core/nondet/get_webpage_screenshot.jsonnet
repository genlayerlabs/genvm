local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/get_webpage_screenshot.py') {
    "calldata": |||
        {
            "method": "main",
            "args": ["text"]
        }
    |||
}
