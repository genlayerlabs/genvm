local lib = require("lib-genvm")
local llm = require("lib-llm")

-- There is no guarantee that different genvm executions will be executed in the same lua VM.
-- Moreover, multiple genvms can be executed in parallel, so avoid using global state.
-- Instead, each genvm creates a session, which has a single `ctx` object,
-- which is preserved across multiple calls

local function just_in_backend(ctx, mapped_prompt, remaining_gen)
	---@cast mapped_prompt MappedPrompt

	local search_in = llm.select_providers_for(mapped_prompt.prompt, mapped_prompt.format)

	lib.log{ prompt = mapped_prompt, search_in = search_in }

	for provider_name, provider_data in pairs(search_in) do
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
		}

		lib.log{level = "trace", message = "calling exec_prompt_in_provider", request = request}
		local success, result = pcall(function ()
			return llm.rs.exec_prompt_in_provider(
				ctx,
				request
			)
		end)

		lib.log{level = "debug", message = "executed with", type = type(result), success = success, result = result}

		if success then
			result.consumed_gen = 0

			return result
		end

		local as_user_error = lib.rs.as_user_error(result)
		if as_user_error == nil then
			lib.log{level = "warning", message = "non-user-error", original = result}

			error(result)
		end

		if llm.overloaded_statuses[as_user_error.ctx.status] then
			lib.log{level = "warning", message = "service is overloaded, looking for next", error = as_user_error}
		else
			lib.log{level = "error", message = "provider failed", error = as_user_error, request = request}

			as_user_error.fatal = true
			lib.rs.user_error(as_user_error)
		end

		::continue::
	end

	lib.log{level = "error", message = "no provider could handle prompt", search_in = search_in}
	lib.rs.user_error({
		causes = {"NO_PROVIDER_FOR_PROMPT"},
		fatal = true,
		ctx = {
			prompt = mapped_prompt.prompt,
			search_in = search_in,
		}
	})
end

function ExecPrompt(ctx, args, remaining_gen)
	---@cast args LLMExecPromptPayload
	---@cast remaining_gen number

	local mapped = llm.exec_prompt_transform(args)

	return just_in_backend(ctx, mapped, remaining_gen)
end

function ExecPromptTemplate(ctx, args, remaining_gen)
	---@cast args LLMExecPromptTemplatePayload
	---@cast remaining_gen number

	local mapped = llm.exec_prompt_template_transform(args)

	return just_in_backend(ctx, mapped, remaining_gen)
end
