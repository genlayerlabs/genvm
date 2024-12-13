local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/ret-tuple.py') {
    "calldata": |||
        {
            "method": "__get_schema__"
        }
    |||
}
