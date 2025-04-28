local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/error_msg_overridden.py') {
    "calldata": |||
        {
            "method": "#error"
        }
    |||,
    message+: {
        value: 100
    }
}
