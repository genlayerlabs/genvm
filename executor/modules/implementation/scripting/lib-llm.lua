local M = {}

local lib = require('lib-genvm')

---@class Prompt
---@field system_message string | nil
---@field user_message string
---@field temperature number
---@field images userdata[]
---@field max_tokens integer
---@field use_max_completion_tokens boolean

---@alias Format "text" | "json" | "bool"

---@alias ModelConfig { enabled: boolean, supports_json: boolean, supports_image: boolean, use_max_completion_tokens: boolean, meta: any }
---@alias ProvidersDB { [string]: { models: { [string]: ModelConfig } } }


---@alias LLMExecPromptPayload { response_format: "text" | "json", prompt: string, images: userdata[] }
---@alias LLMExecPromptTemplatePayload { template: "EqComparative" | "EqNonComparativeValidator" | "EqNonComparativeLeader", vars: table<string, string> }

---@class LLM
---@field exec_prompt_in_provider fun(ctx, data: { prompt: Prompt, format: Format, model: string, provider: string }): any
---@field providers ProvidersDB
---@field templates { eq_comparative: any, eq_non_comparative_leader: any, eq_non_comparative_validator: any }

---@type LLM
local rs = __llm; ---@diagnostic disable-line

M.rs = rs

M.overloaded_statuses = {
	[408] = true,
	[503] = true,
	[429] = true,
	[504] = true,
	[529] = true,
}


M.exec_prompt_in_provider = rs.exec_prompt_in_provider
M.providers = rs.providers
M.templates = rs.templates

---@alias MappedPrompt { prompt: Prompt, format: "json" | "text" | "bool" }

---@return MappedPrompt
M.exec_prompt_transform = function(args)
	local mapped_prompt = {
		system_message = nil,
		user_message = args.prompt,
		temperature = 0.7,
		images = args.images,

		max_tokens = 1000,
		use_max_completion_tokens = false,
	}

	local format = args.response_format

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

local function filter_providers_by(model_fn)
	local ret = {}

	for name, conf in pairs(rs.providers) do
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

---@type ProvidersDB
M.providers_with_json_support = filter_providers_by(function(m) return m.supports_json end)
---@type ProvidersDB
M.providers_with_image_support = filter_providers_by(function(m) return m.supports_image end)
---@type ProvidersDB
M.providers_with_image_and_json_support = filter_providers_by(function(m) return m.supports_image and m.supports_json end)

lib.log{
	providers = M.providers,
	providers_with_json_support = M.providers_with_json_support,
	providers_with_image_support = M.providers_with_image_support,
	providers_with_image_and_json_support = M.providers_with_image_and_json_support,
}

if lib.get_first_from_table(M.providers_with_json_support) == nil then
	lib.log{
		level = "warning",
		message = "no provider with json support detected"
	}
end

if lib.get_first_from_table(M.providers_with_image_support) == nil then
	lib.log{
		level = "warning",
		message = "no provider with image support detected"
	}
end

if lib.get_first_from_table(M.providers_with_image_and_json_support) == nil then
	lib.log{
		level = "error",
		message = "no provider with image AND json support detected"
	}
end

M.select_providers_for = function(prompt, format)
	---@cast prompt Prompt
	---@cast format "text" | "json" | "bool"

	local has_image = lib.get_first_from_table(prompt.images) ~= nil
	if format == 'json' or format == 'bool' then
		if has_image then
			return M.providers_with_image_and_json_support
		else
			return M.providers_with_json_support
		end
	elseif has_image then
		return M.providers_with_image_support
	else
		return M.providers
	end
end

---@return MappedPrompt
M.exec_prompt_template_transform = function(args)
	lib.log{level = "debug", message = "exec_prompt_template_transform", args = args}

	my_data = {
		EqComparative = { template_id = "eq_comparative", format = "bool" },
		EqNonComparativeValidator = { template_id = "eq_non_comparative_validator", format = "bool" },
		EqNonComparativeLeader = { template_id = "eq_non_comparative_leader", format = "text" },
	}

	my_data = my_data[args.template]
	local my_template = M.rs.templates[my_data.template_id]

	args.template = nil
	local vars = args

	local as_user_text = my_template.user
	for key, val in pairs(vars) do
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
