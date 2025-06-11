local simple = import 'templates/simple.jsonnet';
simple.run('${jsonnetDir}/file_schema_web.py')  + {
    "calldata": '{ "args": ["file:///usr/bin/env"] }',
    "message"+: {
        "is_init": true,
    },
}
