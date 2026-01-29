local simple_deploy = import 'templates/simple_deploy.jsonnet';
simple_deploy.run('${jsonnetDir}/${fileBaseName}.py') {
    message+: {
        datetime: "2025-07-29T19:34:20+09:00",
    },
}
