local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/error_msg.py') {
    "calldata": |||
        {
            "method": "#error"
        }
    |||
}
