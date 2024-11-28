local simple = import '../../../templates/simple.jsonnet';
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
            "value": "Rats make great pets due to their affectionate and playful nature. They bond closely with humans, are curious, and enjoy interactive toys. Their intelligence allows them to learn tricks, resembling small dogs, and their charming personalities warrant greater appreciation."
        }
    ]
}
