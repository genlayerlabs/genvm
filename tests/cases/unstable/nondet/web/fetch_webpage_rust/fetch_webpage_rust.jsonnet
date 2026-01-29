local simple = import 'templates/simple_deploy.jsonnet';
simple.run('${jsonnetDir}/fetch_webpage.wasm') {
    "prepare": '${jsonnetDir}/prepare.py',
    "calldata": |||
        {}
    |||
}
