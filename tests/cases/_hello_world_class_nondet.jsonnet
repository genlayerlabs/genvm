local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/${fileBaseName}.py') {
    "calldata": |||
        {
            "method": "foo",
            "args": []
        }
    |||,
    message+: {
        datetime: "2025-07-29T19:34:20+09:00",
    },
}
