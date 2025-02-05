local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/dup-dependency.py') {
    "prepare": '${jsonnetDir}/dup-dependency-prepare.py'
}
