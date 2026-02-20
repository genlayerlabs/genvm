local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/rollback_imm.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
};
local util = import 'templates/util.jsonnet';
{entry: util.addPaths([util.chain([
    s {
        leader_nondet: [
            {
                "kind": "rollback",
                "value": "rollback"
            }
        ]
    },
    s {
        leader_nondet: [
            {
                "kind": "rollback",
                "value": "other rollback"
            }
        ]
    },
])])}
