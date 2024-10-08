local simple = import '../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/send_message.py') {
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
