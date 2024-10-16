local simple = import '../../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/eq_prompt.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
