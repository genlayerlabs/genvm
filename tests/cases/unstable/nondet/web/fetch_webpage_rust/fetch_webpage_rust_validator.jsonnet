local simple = import 'templates/simple_deploy.jsonnet';
local util = import 'templates/util.jsonnet';
{
	prepare: '${jsonnetDir}/prepare_validator.py',
	entry: util.addPaths([simple.run('${jsonnetDir}/fetch_webpage_validator.wasm') {
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
}])
}
