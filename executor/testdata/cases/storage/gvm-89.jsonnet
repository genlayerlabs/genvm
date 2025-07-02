local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/gvm-89.py') {
    "calldata": |||
        {
            "method": "main"
        }
    |||
}
