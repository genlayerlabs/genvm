local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/contract.zip') {
    "prepare": '${jsonnetDir}/prepare.py',
    "calldata": |||
        {
            "method": "main"
        }
    |||
}
