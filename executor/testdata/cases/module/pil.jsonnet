local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/pil.py') {
    "calldata": |||
        {}
    |||
}
