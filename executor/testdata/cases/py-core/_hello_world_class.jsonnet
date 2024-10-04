local simple = import '../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/_hello_world_class.py') {
    "calldata": |||
        {
            "method": "__init__",
            "args": []
        }
    |||
}
