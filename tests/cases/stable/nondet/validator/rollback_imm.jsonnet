local simple = import 'templates/simple.jsonnet';
local s = simple.run('${jsonnetDir}/rollback_imm.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
};
[
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
]
