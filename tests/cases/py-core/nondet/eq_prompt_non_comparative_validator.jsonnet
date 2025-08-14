local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/eq_prompt_non_comparative.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||,
    leader_nondet: [
        {
            "kind": "return",
            "value": "Rats make fantastic pets, being affectionate, intelligent, and playful. They form strong bonds with humans, learn tricks, and possess charming, adaptable personalities."
        }
    ]
}
