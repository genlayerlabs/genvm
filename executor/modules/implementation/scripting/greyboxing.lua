local lib = require("lib-greyboxing")
local inspect = require("inspect")

function just_in_backend(args, prompt, format)
	local search_in = lib.select_backends_for(args, format)

	lib.log{ args = args, prompt = prompt, format = format, search_in = search_in }


	for provider_name, provider_data in pairs(search_in) do
		local model = lib.get_first_from_table(provider_data.models)
		prompt.use_max_completion_tokens = model.value.use_max_completion_tokens

		local success, result = pcall(function ()
			return args.handler:exec_in_backend({
				provider = provider_name,
				model = model.key,
				prompt = prompt,
				format = format,
			})
		end)

		lib.log{message = "executed with", type = type(result), res = result}
		if success then
			return result
		elseif tostring(result):match("runtime error: ([a-zA-Z]*)") == "Overloaded" then
			-- nothing/continue
		else
			error(result)
		end
	end

	error("no provider could handle prompt, searched in " .. inspect(search_in))
end

function exec_prompt(args)
	local handler = args.handler

	local mapped = lib.exec_prompt_transform(args)

	return just_in_backend(args, mapped.prompt, mapped.format)
end

function exec_prompt_template(args)
	local handler = args.handler

	local mapped = lib.exec_prompt_template_transform(args)

	return just_in_backend(args, mapped.prompt, mapped.format)
end
