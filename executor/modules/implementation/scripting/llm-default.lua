local lib = require("lib-genvm")
local llm = require("lib-llm")

local function just_in_backend(ctx, mapped_prompt)
	---@cast mapped_prompt MappedPrompt

	local search_in = llm.select_backends_for(mapped_prompt.prompt, mapped_prompt.format)

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

		local success, result = pcall(function ()
			return llm.rs.exec_prompt_in_provider(
				ctx,
				request
			)
		end)

		lib.log{level = "debug", message = "executed with", type = type(result), result = result}

		if success then
			return result
		end

		local as_user_error = lib.rs.as_user_error(result)
		if as_user_error == nil then
			error(result)
		end

		if result.causes[1] == "OVERLOADED" then
			lib.log{level = "warning", message = "service is overloaded, looking for next", result = result}
		else
			lib.log{level = "error", message = "provider failed", result = result, request = request}

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
