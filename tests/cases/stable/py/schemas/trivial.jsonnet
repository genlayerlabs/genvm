local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/${fileBaseName}.py') {
    "calldata": |||
        {
            "method": "#get-schema"
        }
    |||
}
