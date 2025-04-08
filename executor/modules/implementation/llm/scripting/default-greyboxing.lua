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

function exec_prompt(args)
	local handler = args.handler
	local prompt = args.prompt

	for provider_name, provider_data in pairs(greyboxing.available_backends) do
		local model = provider_data.models[1]

		local success, result = pcall(function ()
			return handler:exec_in_backend({provider = provider_name, model = model, text = prompt})
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
