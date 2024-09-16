local simple = import '../../templates/simple.jsonnet';
simple.run('${jsonnetDir}/call_llm.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
