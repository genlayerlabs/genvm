function dump(o)
	if type(o) == 'table' then
		local s = '{ '
		for k,v in pairs(o) do
			if type(k) ~= 'number' then k = '"'..k..'"' end
			s = s .. '['..k..'] = ' .. dump(v) .. ', '
		 end
		 return s .. '} '
	else
		return tostring(o)
	end
end

function just_in_backend(args, prompt, format)
	for provider_name, provider_data in pairs(greyboxing.available_backends) do
		local model = provider_data.models[1]

		local success, result = pcall(function ()
			return args.handler:exec_in_backend({
				provider = provider_name,
				model = model,
				prompt = prompt,
				format = format,
			})
		end)

		greyboxing.log{message = "executed with", type = type(result), res = dump(result)}
		if success then
			return result
		elseif tostring(result):match("runtime error: ([a-zA-Z]*)") == "Overloaded" then
			-- nothing/continue
		else
			error(result)
		end
	end

	error("no provider could handle prompt")
end

function exec_prompt(args)
	local handler = args.handler

	local mapped_prompt = {
		system_message = nil,
		user_message = args.payload.prompt,
		temperature = 0.7,
	}

	local format = args.payload.response_format

	if format == 'json' then
		mapped_prompt.system_message = "respond with a valid json object"
	end

	return just_in_backend(args, mapped_prompt, format)
end

function exec_prompt_template(args)
	local handler = args.handler

	local template = nil
	local vars = nil

	my_data = {
		EqComparative = { template_id = "eq_comparative", format = "bool" },
		EqNonComparativeValidator = { template_id = "eq_non_comparative_validator", format = "bool" },
		EqNonComparativeLeader = { template_id = "eq_non_comparative_leader", format = "text" },
	}

	my_data = my_data[args.payload.template]
	args.payload.template = nil

	local my_template = greyboxing.templates[my_data.template_id]

	local as_user_text = my_template.user
	for key, val in pairs(args.payload) do
		as_user_text = string.gsub(as_user_text, "#{" .. key .. "}", val)
	end

	local format = my_data.format

	local mapped_prompt = {
		system_message = my_template.system,
		user_message = as_user_text,
		temperature = 0.7,
	}

	return just_in_backend(args, mapped_prompt, format)
end
