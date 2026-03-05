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
				expected_semantics_path: '${jsonnetDir}/${fileBaseName}' + suff + '.stdout',
				[if !std.objectHas(entry, 'expected_semantics_components') then 'expected_semantics_components']: ['stdout', 'return', 'nondet', 'messages'],
			},
		data+: {
			parentPath: treePath,
		},
	};

local expandModes(entry, parentMainTreePath) =
	local modes = std.stringChars(entry.modes);
	local hasLeaderMode = std.member(modes, 'l');
	local hasValidatorMode = std.member(modes, 'v');
	local hasLeaderResult = std.objectHas(entry, 'leader_nondet');
	local is_stable_hash = std.get(entry, 'stable_hash', true);
	local mainMode = if hasLeaderMode then 'l' else if hasValidatorMode then 'v' else 's';
	local mainTreePath = entry.tree_path + mainMode;

	{
		mainTreePath: mainTreePath,
		entries: [
			local result_path = '${tmpDir}/' + entry.tree_path + m + '/result.pickle';
			local isLeader = m == 'l';
			local expected_hash_path =
				if is_stable_hash
				then '${jsonnetDir}/${fileBaseName}.' + entry.tree_path + '.hash'
				else if isLeader
				then null
				else if hasLeaderMode
				then '${tmpDir}/' + entry.tree_path + 'l/hash'
				else if hasValidatorMode
				then '${tmpDir}/' + entry.tree_path + 'v/hash'
				else null
				;
			local base =
				if isLeader
				then entry
				else {[k]: entry[k] for k in std.objectFields(entry) if k != 'next'};
			base + {
				mode: m,
				result_path: result_path,
				expected_hash_path: expected_hash_path,
				tree_path: entry.tree_path + m,
				depends_on: if m == mainMode then parentMainTreePath else mainTreePath,
			} +
				if m != mainMode
				then
				{ expected_semantics_components: [] }
				else {}
			for m in modes
		],
	};

local recurse(entries, data) =
	std.flatMap(function(i)
		local res = std.foldl((function(acc, nxt) nxt(acc.entry, acc.data)), [addOutPaths], {
			data: data {i: i},
			entry: entries[i],
		});

		local entry = res.entry;
		local entryData = res.data;
		local expanded = expandModes(entry, data.parentMainTreePath);

		[
			e + (
				if std.objectHas(e, 'next') then {
					next: recurse(e.next, entryData { parentMainTreePath: expanded.mainTreePath })
				} else {}
			)
			for e in expanded.entries
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

	mapGraph(fn, entries)::
		[
			local mapped = fn(entries[i]);
			mapped + (
				if std.objectHas(mapped, 'next')
				then { next: $.mapGraph(fn, mapped.next) }
				else {}
			)
			for i in std.range(0, std.length(entries) - 1)
		],

	addPaths(entries):: recurse(entries, {
		parentPath: null,
		parentMainTreePath: null,
	}),
}
