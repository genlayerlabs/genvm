local addOutPaths(entry, data) =
	local parentPath = data.parentPath;
	local treePath = if parentPath == null then std.toString(data.i) else parentPath + '_' + std.toString(data.i);
	local suff = '.' + treePath;
	{
		entry:
			entry + {
				[if !std.objectHas(entry, 'modes') then 'modes']: 'lvs',
				[if !std.objectHas(entry, 'verify_against_path') then 'verify_against_path']: null,
				expected_semantics_path: '${jsonnetDir}/${fileBaseName}' + suff + '.stdout',
				[if !std.objectHas(entry, 'expected_semantics_components') then 'expected_semantics_components']: ['stdout', 'return', 'nondet', 'messages'],
				expected_hash_parts: '${jsonnetDir}/${fileBaseName}' + suff + '.hash',
				result_path: '${tmpDir}/' + treePath + '/result.pickle',
			},
		data+: {
			parentPath: treePath,
		},
	};

local recurse(entries, data) =
[
	local res = std.foldl((function(acc, nxt) nxt(acc.entry, acc.data)), [addOutPaths], {
		data: data {i:i},
		entry: entries[i],
	});

	local entry = res.entry;
	local data = res.data;

	entry + (
		if std.objectHas(entry, 'next') then {
			next: recurse(entry.next, data)
		} else {}
	)

	for i in std.range(0, std.length(entries) - 1)
];

{
	chain(steps)::
		if std.length(steps) == 1 then steps[0]
		else steps[0] + {next: [$.chain(steps[1:])]},

	updateArrayElement(arr, i, fn)::
		[if j == i then fn(arr[j]) else arr[j] for j in std.range(0, std.length(arr) - 1)],

	updateField(obj, field, fn)::
		obj + {[field]: fn(obj[field])},

	addPaths(entries):: recurse(entries, {
		parentPath: null
	}),
}
