local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/eq_prompt_comparative.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
