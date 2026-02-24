local addOutPaths(entry, data) =
	local parentPath = data.parentPath;
	local treePath = if parentPath == null then std.toString(data.i) else parentPath + '_' + std.toString(data.i);
	local suff = '.' + treePath;
	{
		entry:
			entry + {
				tree_path: treePath,
				parent_tree_path: parentPath,
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

local expandModes(entry) =
	local modes = std.stringChars(entry.modes);
	local hasLeader = std.member(modes, 'l');
	local leaderHashPath = entry.expected_hash_parts;
	local leaderNondetPath = std.strReplace(entry.result_path, '/result.pickle', '/leader_nondet.pickle');
	[
		local isLeader = m == 'l';
		local base = if isLeader then entry
			else {[k]: entry[k] for k in std.objectFields(entry) if k != 'next'};
		base + {
			mode: m,
		} + (
			if !isLeader then {
				tree_path: entry.tree_path + '.' + m,
				expected_hash_parts: std.strReplace(entry.expected_hash_parts, '.hash', '.' + m + '.hash'),
				result_path: std.strReplace(entry.result_path, '/result.pickle', '.' + m + '/result.pickle'),
				expected_semantics_components: [],
			} else {}
		) + (
			if !isLeader && hasLeader then {
				verify_against_path: leaderHashPath,
				leader_nondet_path: leaderNondetPath,
			} else {}
		)
		for m in modes
	];

local recurse(entries, data) =
	std.flatMap(function(i)
		local res = std.foldl((function(acc, nxt) nxt(acc.entry, acc.data)), [addOutPaths], {
			data: data {i: i},
			entry: entries[i],
		});

		local entry = res.entry;
		local entryData = res.data;
		local expanded = expandModes(entry);

		[
			e + (
				if std.objectHas(e, 'next') then {
					next: recurse(e.next, entryData)
				} else {}
			)
			for e in expanded
		]
	, std.range(0, std.length(entries) - 1));

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
