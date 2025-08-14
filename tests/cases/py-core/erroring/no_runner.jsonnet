local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/no_runner.py') {
    "calldata": |||
        {
        }
    |||
}
