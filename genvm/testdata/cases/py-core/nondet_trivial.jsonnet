local simple = import '../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/nondet_trivial.py') {
    "calldata": |||
        {
            "method": "init",
            "args": []
        }
    |||
}
