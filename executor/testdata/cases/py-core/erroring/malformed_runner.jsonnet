local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/malformed_runner.py') {
    "calldata": |||
        {
        }
    |||
}
