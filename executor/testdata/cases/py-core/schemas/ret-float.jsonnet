local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/ret-float.py') {
    "calldata": |||
        {
            "method": "__get_schema__"
        }
    |||
}
