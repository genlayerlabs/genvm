local M = {}

M.all_backends = greyboxing.available_backends
M.sleep_seconds = greyboxing.sleep_seconds

local inspect = require('inspect')
local value2json = require('value2json')

M.log = function(arg)
	greyboxing.log(value2json(arg))
end

M.get_first_from_table = function(t)
	if t == nil then
		return nil
	end

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
		images = args.payload.images,

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

local function shallow_copy(t)
	local ret = {}
	for k, v in pairs(t) do
		ret[k] = v
	end
	return ret
end

function filter_backends_by(model_fn)
	local ret = {}

	for name, conf in pairs(greyboxing.available_backends) do
		local cur = shallow_copy(conf)
		cur.models = {}

		local has = false
		for model_name, model_data in pairs(conf.models) do
			if model_fn(model_data) then
				cur.models[model_name] = model_data
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

M.log{
	all_backends = M.all_backends,
	backends_with_json_support = M.backends_with_json_support,
	backends_with_image_support = M.backends_with_image_support,
	backends_with_image_and_json_support = M.backends_with_image_and_json_support,
}

if M.get_first_from_table(M.backends_with_json_support) == nil then
	M.log{
		level = "warning",
		message = "no backend with json support detected"
	}
end

if M.get_first_from_table(M.backends_with_image_support) == nil then
	M.log{
		level = "warning",
		message = "no backend with image support detected"
	}
end

if M.get_first_from_table(M.backends_with_image_and_json_support) == nil then
	M.log{
		level = "error",
		message = "no backend with image AND json support detected"
	}
end

M.exec_in_backend = function(handler, x)
	local success, result = pcall(function() return handler:exec_in_backend(x) end)

	if not success then
		error({
			kind = "LuaError",
			ctx = {
				original_error = result
			}
		})
	end

	if result.Err ~= nil then
		error(result.Err)
	end

	return result.Ok
end

M.select_backends_for = function(args, format)
	local has_image = M.get_first_from_table(args.payload.images) ~= nil
	if format == 'json' or format == 'bool' then
		if has_image then
			return M.backends_with_image_and_json_support
		else
			return M.backends_with_json_support
		end
	elseif has_image then
		return M.backends_with_image_support
	else
		return M.all_backends
	end
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
		images = {},
		max_tokens = 1000,
		use_max_completion_tokens = false,
	}

	return {
		prompt = mapped_prompt,
		format = format
	}
end

return M
