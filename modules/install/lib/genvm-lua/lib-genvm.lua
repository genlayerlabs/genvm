local M = {}

local value2json = require("value2json")

---@class ModuleError
---@field causes string[]
---@field fatal boolean
---@field ctx table<string, any>

---@class RS
---@field log_json fun(val: any): nil
---@field sleep_seconds fun(duration: number): nil
---@field request
---| fun(ctx, req: { body: nil | string, url: string, headers: table<string, string>, method: string, error_on_status: boolean | nil, json: false | nil, response_body_max_size: integer | nil }): { body: string, status: integer, headers: table<string, string> }
---| fun(ctx, req: { body: nil | string, url: string, headers: table<string, string>, method: string, error_on_status: boolean | nil, json: true, response_body_max_size: integer | nil }): { body: any, status: integer, headers: table<string, string> }
---@field split_url fun(url: string): nil | { schema: string, port: number | nil, host: string }
---@field user_error fun(val: ModuleError): nil
---@field base64_decode fun(val: string): string
---@field base64_encode fun(val: string): string
---@field json_parse fun(val: string): any
---@field json_stringify fun(val: any): string
---@field as_user_error fun(val: any): nil | ModuleError
---@field url_encode fun(url: string): string
---@field filter_text fun(text: string, filters: string[]): string
---@field filter_image fun(image: string, filters: any[]): string
---@field random_bytes fun(length: integer): string
---@field random_float fun(): number

---@type RS
M.rs = __dflt ---@diagnostic disable-line

--- Log a message or structured data. The argument table may contain `level` and `message` fields.
---@param arg any value to log; converted to JSON via `value2json` before sending
M.log = function(arg)
	M.rs.log_json(value2json(arg))
end

--- Return the first key-value pair from a table, or nil if the table is nil/empty.
---@param t table|nil
---@return { key: any, value: any }|nil
M.get_first_from_table = function(t)
	if t == nil then
		return nil
	end

	for k, v in pairs(t) do
		return { key = k, value = v }
	end
	return nil
end

--- Return the first key-value pair from a table; errors if the table is nil or empty.
---@param t table|nil
---@return { key: any, value: any }
M.get_first_from_table_assert = function(t)
	local res = M.get_first_from_table(t)
	if res == nil then
		error("expected non-empty table")
	end

	return res
end

--- Re-raise an error with a different fatality flag. Non-user errors are re-raised as-is.
---@param e any the caught error value
---@param new_fatality boolean new value for the `fatal` field
M.reraise_with_fatality = function(e, new_fatality)
	local err = M.rs.as_user_error(e)
	if err == nil then
		error(e)
	end

	err.fatal = new_fatality
	M.rs.user_error(err)
end

--- Linearly map a value in [0, 1] to [range_min, range_max].
---@param value_01 number normalized value in the 0..1 range
---@param range_min number lower bound of the target range
---@param range_max number upper bound of the target range
---@return number
M.map_01_to_range = function(value_01, range_min, range_max)
	return range_min + (range_max - range_min) * value_01
end

return M
