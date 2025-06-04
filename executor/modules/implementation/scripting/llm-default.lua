local lib = require("lib-genvm")
local llm = require("lib-llm")

local function just_in_backend(ctx, mapped_prompt)
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
			return result
		end

		local as_user_error = lib.rs.as_user_error(result)
		lib.log{level = "warning", message = "error as user", type = type(as_user_error), as_user_error = as_user_error}
		if as_user_error == nil then
			error(result)
		end

		if llm.overloaded_statuses[as_user_error.ctx.status] then
			lib.log{level = "warning", message = "service is overloaded, looking for next", error = as_user_error}
		else
			lib.log{level = "error", message = "provider failed", error = as_user_error, request = request}

			lib.rs.user_error(result)
		end

		::continue::
	end

	lib.log{level = "error", message = "no provider could handle prompt", search_in = search_in}
end

function exec_prompt(ctx, args)
	---@cast args LLMExecPromptPayload

	local mapped = llm.exec_prompt_transform(args)

	return just_in_backend(ctx, mapped)
end

function exec_prompt_template(ctx, args)
	---@cast args LLMExecPromptTemplatePayload

	local mapped = llm.exec_prompt_template_transform(args)

	return just_in_backend(ctx, mapped)
end
