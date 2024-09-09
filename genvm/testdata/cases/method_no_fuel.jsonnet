local simple = import '../templates/simple.jsonnet';
simple.run('${jsonnetDir}/methods.py') {
    "message": super.message + {
        "gas": 100,
    },
    "calldata": |||
        {
            "method": "pub",
            "args": []
        }
    |||
}
