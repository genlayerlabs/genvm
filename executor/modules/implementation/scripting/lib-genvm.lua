local M = {}

local value2json = require('value2json')

---@alias ModuleError { causes: string[], fatal: boolean, ctx: table<string, any> }

---@class RS
---@field log_json fun(val: any): nil
---@field sleep_seconds fun(duration: number): nil
---@field request
---| fun(ctx, req: { body: nil | string, url: string, headers: table<string, string>, method: string, error_on_status: boolean | nil, json: false | nil }): { body: string, status: integer, headers: table<string, string> }
---| fun(ctx, req: { body: nil | string, url: string, headers: table<string, string>, method: string, error_on_status: boolean | nil, json: true }): { body: any, status: integer, headers: table<string, string> }
---@field split_url fun(url: string): nil | { schema: string, port: number | nil, host: string }
---@field user_error fun(val: ModuleError): nil
---@field base64_decode fun(val: string): string
---@field base64_encode fun(val: string): string
---@field json_parse fun(val: string): any
---@field json_stringify fun(val: any): string
---@field as_user_error fun(val: any): nil | ModuleError

---@type RS
M.rs = __dflt;  ---@diagnostic disable-line

-- supports "level" and "message"
M.log = function(arg)
	M.rs.log_json(value2json(arg))
end

-- for nil and empty table returns nil
M.get_first_from_table = function(t)
	if t == nil then
		return nil
	end

	for k, v in pairs(t) do
		return { key = k, value = v }
	end
	return nil
end

M.get_first_from_table_assert = function(t)
	local res = M.get_first_from_table(t)
	if res == nil then
		error("expected non-empty table")
	end

	return res
end

M.reraise_with_fatality = function(e, new_fatality)
	local err = M.rs.as_user_error(e)
	if err == nil then
		error(e)
	end

	err.fatal = new_fatality
	M.rs.user_error(err)
end

return M
