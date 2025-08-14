local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/call_llm_json.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
