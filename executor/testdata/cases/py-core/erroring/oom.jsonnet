local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/oom.py') {
    "calldata": |||
        {
        }
    |||
}
