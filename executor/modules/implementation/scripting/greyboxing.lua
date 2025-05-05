local lib = require("lib-greyboxing")
local inspect = require("inspect")

function just_in_backend(args, prompt, format)
	local search_in = lib.select_backends_for(args, format)

	for provider_name, provider_data in pairs(lib.all_backends) do
		local model = provider_data.models[1]

		local success, result = pcall(function ()
			return args.handler:exec_in_backend({
				provider = provider_name,
				model = model,
				prompt = prompt,
				format = format,
			})
		end)

		lib.log{message = "executed with", type = type(result), res = inspect(result)}
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
