local lib = require("lib-genvm")
local llm = require("lib-llm")
local sqlite3 = require("lsqlite3")

-- There is no guarantee that different genvm executions will be executed in the same lua VM.
-- Moreover, multiple genvms can be executed in parallel, so avoid using global state.
-- Instead, each genvm creates a session, which has a single `ctx` object,
-- which is preserved across multiple calls

local function get_or_create_stats(ctx, provider, model)
	local key = provider .. "/" .. model
	local entry = ctx.stats[key]
	if entry == nil then
		entry = {
			error_count = 0,
			input_tokens = 0,
			output_tokens = 0,
			total_tokens = 0,
			cache_read_tokens = 0,
			cache_write_tokens = 0,
			image_units = 0,
		}
		ctx.stats[key] = entry
	end
	return entry
end

---@param request { prompt: Prompt, format: Format, model: string, provider: string, timeout: number | nil }
---@param calc_consumed_gen fun(res: ProviderResult): Rat | number | nil
---@return ProviderResult
local function exec_update_policy_data(ctx, request, calc_consumed_gen)
	local success, res = pcall(llm.rs.exec_prompt_in_provider, ctx, request)

	local entry = get_or_create_stats(ctx, request.provider, request.model)

	if success then
		local consumed_gen = calc_consumed_gen(res)
		if consumed_gen then
			ctx.policy.spent_gen_wei = ctx.policy.spent_gen_wei + rat.new(consumed_gen)
		end
		if ctx.policy.stop_on_spent and ctx.policy.spent_gen_wei >= ctx.policy.stop_on_spent then
			ctx.policy.exhausted = true
		end

		local t = res.tokens
		if t then
			entry.input_tokens = entry.input_tokens + (t.input or 0)
			entry.output_tokens = entry.output_tokens + (t.output or 0)
			entry.total_tokens = entry.total_tokens + (t.total or 0)
			entry.cache_read_tokens = entry.cache_read_tokens + (t.cache_read or 0)
			entry.cache_write_tokens = entry.cache_write_tokens + (t.cache_write or 0)
			entry.image_units = entry.image_units + (t.image_units or 0)
		end
		return res
	else
		entry.error_count = entry.error_count + 1

		error(res)
	end
end

local MAX_BUDGET_TIMEOUT = rat.new("60")

local function compute_timeout(ctx, remaining_gen)
	if ctx.gen_per_time_unit == nil then
		return nil
	end
	local timeout = remaining_gen / ctx.gen_per_time_unit * rat.new("3/2")
	if timeout <= rat.zero or timeout > MAX_BUDGET_TIMEOUT then
		return nil
	end
	return timeout:to_float()
end

local function just_in_backend(ctx, mapped_prompt, remaining_gen)
	---@cast mapped_prompt MappedPrompt

	local search_in = llm.select_providers_for(mapped_prompt.prompt, mapped_prompt.format)

	lib.log {
		level = "debug",
		message = "executing prompt in backend",
		prompt = mapped_prompt,
		search_in = search_in,
	}

	local timeout = compute_timeout(ctx, remaining_gen)
	if timeout and timeout < 1 then
		lib.log {
			level = "warning",
			message = "computed timeout is very low, failing immediately",
			timeout = timeout,
		}
		llm.exhaust()
	end

	local provider_keys = {}
	for provider_name, _ in pairs(search_in) do
		table.insert(provider_keys, provider_name)
	end

	table.sort(provider_keys, function(a, b)
		local a_data = search_in[a]
		local b_data = search_in[b]

		local a_priority = a_data.meta and a_data.meta.priority or 0
		local b_priority = b_data.meta and b_data.meta.priority or 0

		if a_priority ~= b_priority then
			return a_priority > b_priority
		end
		return a > b -- just compare names
	end)

	for _, provider_name in ipairs(provider_keys) do
		local provider_data = search_in[provider_name]
		local model = lib.get_first_from_table(provider_data.models)

		if model == nil then
			goto continue
		end

		mapped_prompt.prompt.use_max_completion_tokens = model.value.use_max_completion_tokens

		local request = {
			provider = provider_name,
			model = model.key,
			prompt = mapped_prompt.prompt,
			format = mapped_prompt.format,
			timeout = timeout,
		}

		lib.log {
			level = "trace",
			message = "calling exec_prompt_in_provider",
			request = request,
		}
		local success, result = pcall(exec_update_policy_data, ctx, request, function(res)
			-- TODO: here we need to convert tokens into
			-- gen
			return res.consumed_gen
		end)

		lib.log {
			level = "debug",
			message = "executed with",
			type = type(result),
			success = success,
			result = result,
		}

		if success then
			return result
		end

		local as_user_error = lib.rs.as_user_error(result)
		if as_user_error == nil then
			lib.log { level = "warning", message = "non-user-error", original = result }

			error(result)
		end

		if llm.overloaded_statuses[as_user_error.ctx.status] then
			lib.log { level = "warning", message = "service is overloaded, looking for next", error = as_user_error }
		else
			lib.log { level = "error", message = "provider failed", error = as_user_error, request = request }
			-- lets fall back to retry
			-- as_user_error.fatal = true
			-- lib.rs.user_error(as_user_error)
		end

		::continue::
	end

	lib.log { level = "error", message = "no provider could handle prompt", search_in = search_in }
	lib.rs.user_error {
		causes = { "NO_PROVIDER_FOR_PROMPT" },
		fatal = true,
		ctx = {
			prompt = mapped_prompt.prompt,
			search_in = search_in,
		},
	}
end

function Setup(ctx)
	ctx.stats = {}

	local gen_per_time_unit_str = ctx.gas_data and ctx.gas_data.genPerTimeUnit
	local gen_per_time_unit = nil
	if gen_per_time_unit_str then
		local r = rat.new(gen_per_time_unit_str)
		if not r:is_zero() then
			gen_per_time_unit = r
		end
	end
	ctx.gen_per_time_unit = gen_per_time_unit

	local stop_on_spent = nil
	if gen_per_time_unit then
		stop_on_spent = gen_per_time_unit * rat.new(ctx.initial_time_units_allocation)
	end

	ctx.policy = {
		spent_gen_wei = rat.zero,
		exhausted = false,
		stop_on_spent = stop_on_spent,
	}
end

function Teardown(ctx)
	local data_dir = lib.rs.data_dir

	local has_stats = false
	for _ in pairs(ctx.stats) do
		has_stats = true
		break
	end
	if not has_stats then
		return
	end

	local db_path = data_dir .. "/stats.sqlite"
	local db = sqlite3.open(db_path)

	db:exec([[
		CREATE TABLE IF NOT EXISTS provider_stats (
			provider_model TEXT PRIMARY KEY,
			error_count INTEGER NOT NULL DEFAULT 0,
			input_tokens INTEGER NOT NULL DEFAULT 0,
			output_tokens INTEGER NOT NULL DEFAULT 0,
			total_tokens INTEGER NOT NULL DEFAULT 0,
			cache_read_tokens INTEGER NOT NULL DEFAULT 0,
			cache_write_tokens INTEGER NOT NULL DEFAULT 0,
			image_units INTEGER NOT NULL DEFAULT 0
		)
	]])

	local stmt = db:prepare([[
		INSERT INTO provider_stats (
			provider_model, error_count,
			input_tokens, output_tokens, total_tokens,
			cache_read_tokens, cache_write_tokens, image_units
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(provider_model) DO UPDATE SET
			error_count = error_count + excluded.error_count,
			input_tokens = input_tokens + excluded.input_tokens,
			output_tokens = output_tokens + excluded.output_tokens,
			total_tokens = total_tokens + excluded.total_tokens,
			cache_read_tokens = cache_read_tokens + excluded.cache_read_tokens,
			cache_write_tokens = cache_write_tokens + excluded.cache_write_tokens,
			image_units = image_units + excluded.image_units
	]])

	for key, entry in pairs(ctx.stats) do
		stmt:bind_values(
			key,
			entry.error_count,
			entry.input_tokens,
			entry.output_tokens,
			entry.total_tokens,
			entry.cache_read_tokens,
			entry.cache_write_tokens,
			entry.image_units
		)
		stmt:step()
		stmt:reset()
	end

	stmt:finalize()
	db:close()
end

function ExecPrompt(ctx, args, remaining_gen)
	---@cast args LLMExecPromptPayload
	---@cast remaining_gen number

	if ctx.policy.exhausted then
		llm.exhaust()
	end

	args.prompt = lib.rs.filter_text(args.prompt, {
		"NFKC",
		"RmZeroWidth",
		"NormalizeWS",
	})

	if args.prompt == "" then
		lib.rs.user_error {
			causes = { "EMPTY_PROMPT" },
			fatal = false,
			ctx = {},
		}
	end

	local mapped = llm.exec_prompt_transform(args)

	return just_in_backend(ctx, mapped, remaining_gen)
end

function ExecPromptTemplate(ctx, args, remaining_gen)
	---@cast args LLMExecPromptTemplatePayload
	---@cast remaining_gen number

	if ctx.policy.exhausted then
		return llm.exhaust()
	end

	local mapped = llm.exec_prompt_template_transform(args)

	return just_in_backend(ctx, mapped, remaining_gen)
end
