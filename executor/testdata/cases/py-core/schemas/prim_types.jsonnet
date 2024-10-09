local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/prim_types.py') {
    "calldata": |||
        {
            "method": "__get_schema__"
        }
    |||
}
