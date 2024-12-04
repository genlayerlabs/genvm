local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/complex_types.py') {
    "calldata": |||
        {
            "method": "__get_schema__"
        }
    |||
}
