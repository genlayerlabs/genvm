local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/call_llm_imgs.py') {
    "calldata": |||
        {
            "method": "main",
            "args": []
        }
    |||
}
