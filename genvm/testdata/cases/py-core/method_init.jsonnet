local simple = import '../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/methods.py') {
    "calldata": |||
        {
            "method": "__init__",
            "args": []
        }
    |||,
    "message": super.message + {
        "is_init": true,
    }
}
