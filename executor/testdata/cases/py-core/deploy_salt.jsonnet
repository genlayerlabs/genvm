local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/deploy_salt.py') {
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
