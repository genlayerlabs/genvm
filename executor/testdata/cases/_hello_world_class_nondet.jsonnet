local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/_hello_world_class_nondet.py') {
    "calldata": |||
        {
            "method": "foo",
            "args": []
        }
    |||
}
