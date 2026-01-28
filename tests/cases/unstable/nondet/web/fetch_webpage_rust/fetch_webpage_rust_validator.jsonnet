local simple = import 'templates/simple_deploy.jsonnet';
simple.run('${jsonnetDir}/fetch_webpage_validator.wasm') {
    "prepare": '${jsonnetDir}/prepare_validator.py',
    "calldata": |||
        {}
    |||,
    sync: false,
    leader_nondet: [
        {
            "kind": "return",
            "value": "Hello world!"
        }
    ]
}
