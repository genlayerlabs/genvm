local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/trivial.py') {
    "calldata": |||
        {
            "method": "__get_schema__"
        }
    |||
}
