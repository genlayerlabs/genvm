local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/${fileBaseName}.py') {
    "calldata": |||
        {}
    |||,
    "message": super.message + {
        "is_init": true,
    }
}
