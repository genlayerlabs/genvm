local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/pub_ctor.py') {
    "calldata": |||
        {
            "method": "__init__",
            "args": []
        }
    |||,
    message+: {
        "is_init": true
    }
}
