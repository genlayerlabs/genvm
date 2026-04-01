local M = {}

local lib = require("lib-genvm")

---@alias MergeStrategy "none" | "replace" | "merge_left" | "merge_right" | table<string, MergeStrategy>

---@class Prompt
---@field system_message string | nil
---@field user_message string
---@field temperature number
---@field images userdata[]
---@field max_tokens integer
---@field use_max_completion_tokens boolean
---@field seed integer | nil
---@field extra table<string, any> | nil
---@field extra_merge_strategy MergeStrategy | nil

---@alias Format "text" | "json" | "bool"

---@class ModelConfig
---@field enabled boolean
---@field supports_json boolean
---@field supports_image boolean
---@field use_max_completion_tokens boolean
---@field meta any

---@class ProviderEntry
---@field models table<string, ModelConfig>
---@field meta any

---@alias ProvidersDB table<string, ProviderEntry>

---@class LLMExecPromptPayload
---@field response_format "text" | "json"
---@field prompt string
---@field images userdata[]

--- Additional string fields serve as template variable substitutions.
---@class LLMExecPromptTemplatePayload
---@field template "EqComparative" | "EqNonComparativeValidator" | "EqNonComparativeLeader"
---@field [string] string

---@class LLM
---@field exec_prompt_in_provider fun(ctx, data: { prompt: Prompt, format: Format, model: string, provider: string }): any
---@field providers ProvidersDB
---@field templates { eq_comparative: any, eq_non_comparative_leader: any, eq_non_comparative_validator: any }

---@type LLM
local rs = __llm ---@diagnostic disable-line

M.rs = rs

--- HTTP status codes that indicate a provider is overloaded and the request may be retried.
---@type table<integer, boolean>
M.overloaded_statuses = {
	[408] = true,
	[503] = true,
	[429] = true,
	[504] = true,
	[529] = true,
}

--- Execute a prompt against a specific provider/model. Delegates to the runtime.
---@type fun(ctx, data: { prompt: Prompt, format: Format, model: string, provider: string }): any
M.exec_prompt_in_provider = rs.exec_prompt_in_provider

--- Full provider database (all providers and models).
---@type ProvidersDB
M.providers = rs.providers

--- Prompt templates keyed by equivalence-check type.
---@type { eq_comparative: any, eq_non_comparative_leader: any, eq_non_comparative_validator: any }
M.templates = rs.templates

---@class MappedPrompt
---@field prompt Prompt
---@field format "json" | "text" | "bool"

--- Transform raw prompt arguments into a MappedPrompt suitable for provider execution.
--- Sets default temperature (0.7) and max_tokens (8000). For JSON format, adds a system message.
---@param args { prompt: string, images: userdata[], response_format: Format }
---@return MappedPrompt
M.exec_prompt_transform = function(args)
	local mapped_prompt = {
		system_message = nil,
		user_message = args.prompt,
		images = args.images,

		temperature = 0.7,
		max_tokens = 8000,
		use_max_completion_tokens = false,
		seed = nil,
	}

	local format = args.response_format

	if format == "json" then
		mapped_prompt.system_message = "respond with a valid json object"
	end

	return {
		prompt = mapped_prompt,
		format = format,
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

--- Providers whose models support JSON response format.
---@type ProvidersDB
M.providers_with_json_support = filter_providers_by(function(m)
	return m.supports_json
end)

--- Providers whose models support image inputs.
---@type ProvidersDB
M.providers_with_image_support = filter_providers_by(function(m)
	return m.supports_image
end)

--- Providers whose models support both image inputs and JSON response format.
---@type ProvidersDB
M.providers_with_image_and_json_support = filter_providers_by(function(m)
	return m.supports_image and m.supports_json
end)

lib.log {
	providers = M.providers,
	providers_with_json_support = M.providers_with_json_support,
	providers_with_image_support = M.providers_with_image_support,
	providers_with_image_and_json_support = M.providers_with_image_and_json_support,
}

if lib.get_first_from_table(M.providers_with_json_support) == nil then
	lib.log {
		level = "error",
		message = "no provider with json support detected",
	}
end

if lib.get_first_from_table(M.providers_with_image_support) == nil then
	lib.log {
		level = "warning",
		message = "no provider with image support detected",
	}
end

if lib.get_first_from_table(M.providers_with_image_and_json_support) == nil then
	lib.log {
		level = "warning",
		message = "no provider with image AND json support detected",
	}
end

--- Select the appropriate provider subset for a given prompt and response format.
--- Filters by JSON support, image support, or both depending on the format and whether the prompt contains images.
---@param prompt Prompt
---@param format Format
---@return ProvidersDB
M.select_providers_for = function(prompt, format)
	---@cast prompt Prompt
	---@cast format "text" | "json" | "bool"

	local has_image = lib.get_first_from_table(prompt.images) ~= nil
	if format == "json" or format == "bool" then
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

--- Transform a template-based prompt into a MappedPrompt by substituting variables into the template text.
--- Looks up the template by `args.template` and replaces `#{key}` placeholders with the remaining fields in `args`.
---@param args LLMExecPromptTemplatePayload
---@return MappedPrompt
M.exec_prompt_template_transform = function(args)
	lib.log { level = "debug", message = "exec_prompt_template_transform", args = args }

	my_data = {
		EqComparative = { template_id = "eq_comparative", format = "bool" },
		EqNonComparativeValidator = { template_id = "eq_non_comparative_validator", format = "bool" },
		EqNonComparativeLeader = { template_id = "eq_non_comparative_leader", format = "text" },
	}

	my_data = my_data[args.template]
	local my_template = M.rs.templates[my_data.template_id]

	local vars = shallow_copy(args)
	vars.template = nil

	local as_user_text = my_template.user
	for key, val in pairs(vars) do
		local val_escaped = string.gsub(val, "%%", "%%%%")
		as_user_text = string.gsub(as_user_text, "#{" .. key .. "}", val_escaped)
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
		format = format,
	}
end

return M
