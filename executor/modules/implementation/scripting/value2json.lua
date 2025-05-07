local function prepare(value, ctx)
	local typ = type(value)

	if typ ~= "table" then
		return
	end

	if ctx.counts[value] == nil then
		ctx.counts[value] = 0
	end

	ctx.counts[value] = ctx.counts[value] + 1

	for k, v in pairs(value) do
		prepare(value[k], ctx)
	end
end

local function get_or_create_id(value, ctx)
	local my_id = ctx.table_ids[value]
	if my_id == nil then
		my_id = ctx.table_next_id
		ctx.table_next_id = ctx.table_next_id + 1
		ctx.table_ids[value] = my_id
	end

	return my_id
end

local function transform(value, ctx, include_ids)
	local typ = type(value)

	if typ == "userdata" then
		if ctx.userdata_ids[value] == nil then
			ctx.userdata_ids[value] = ctx.userdata_next_id
			ctx.userdata_next_id = ctx.userdata_next_id + 1
		end
		return {
			["$userdata"] = ctx.userdata_ids[value]
		}
	end

	if typ ~= "table" then
		return value
	end

	local my_id = nil
	if ctx.counts[value] > 1 or include_ids then
		my_id = get_or_create_id(value, ctx)

		include_ids = true
	end

	if ctx.stack_traced[value] then
		my_id = get_or_create_id(value, ctx)

		return {
			["$table"] = my_id,
		}
	end

	ctx.stack_traced[value] = true

	local dup_table = {}

	for k, v in pairs(value) do
		dup_table[k] = transform(v, ctx)
	end

	local add_id = ctx.table_ids[value]
	if add_id ~= nil then
		dup_table["$id"] = add_id
	end

	ctx.stack_traced[value] = false

	return dup_table
end

local function impl(value)
	local ctx = {
		counts = {},
		userdata_ids = {},
		userdata_next_id = 0,
		table_ids = {},
		table_next_id = 0,
		visited = {},
	}

	prepare(value, ctx)

	ctx.visited = {}
	ctx.stack_traced = {}

	return transform(value, ctx)
end

return impl
