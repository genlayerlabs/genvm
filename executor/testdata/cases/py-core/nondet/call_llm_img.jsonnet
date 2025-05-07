local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/call_llm_img.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
