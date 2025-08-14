local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/complex_types.py') {
    "calldata": |||
        {
            "method": "#get-schema"
        }
    |||
}
