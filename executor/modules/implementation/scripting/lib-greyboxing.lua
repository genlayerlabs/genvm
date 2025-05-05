local M = {}

M.all_backends = greyboxing.available_backends

local inspect = require('inspect')

M.log = function(arg)
	greyboxing.log(inspect(arg))
end

M.get_first_from_table = function(t)
	for k, v in pairs(t) do
		return { key = k, value = v }
	end
	return nil
end

M.exec_prompt_transform = function(args)
	local handler = args.handler

	local mapped_prompt = {
		system_message = nil,
		user_message = args.payload.prompt,
		temperature = 0.7,
		image = args.payload.image,

		max_tokens = 1000,
		use_max_completion_tokens = false,
	}

	local format = args.payload.response_format

	if format == 'json' then
		mapped_prompt.system_message = "respond with a valid json object"
	end

	return {
		prompt = mapped_prompt,
		format = format
	}
end

function filter_backends_by(model_fn)
	local ret = {}

	for name, conf in pairs(greyboxing.available_backends) do
		local cur = {}
		local has = false
		for model_name, model_data in pairs(conf) do
			if model_fn(model_data) then
				cur[model_name] = model_data
				has = true
			end
		end

		if has then
			ret[name] = cur
		end
	end

	return ret
end

M.backends_with_json_support = filter_backends_by(function(m) return m.supports_json end)
M.backends_with_image_support = filter_backends_by(function(m) return m.supports_image end)
M.backends_with_image_and_json_support = filter_backends_by(function(m) return m.supports_image and m.supports_json end)

M.select_backends_for = function(args, format)
	if format == 'json' or format == 'bool' then
		if args.image ~= nil then
			return M.backends_with_image_and_json_support
		else
			return M.backends_with_json_support
		end
	elseif args.image ~= nil then
		return M.backends_with_image_support
	end

	return M.all_backends
end

M.exec_prompt_template_transform = function(args)
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
		image = args.payload.image,
		max_tokens = 1000,
		use_max_completion_tokens = false,
	}

	return {
		prompt = mapped_prompt,
		format = format
	}
end

return M
