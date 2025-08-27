local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/../py/other/meth/methods.py') {
    "calldata": |||
        {
            "method": "det_viol",
            "args": []
        }
    |||
}
