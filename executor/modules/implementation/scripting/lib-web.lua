local M = {}

---@alias WebRenderPayload { url: string, mode: "text" | "html" | "screenshot", wait_after_loaded: number }
---@alias WebRequestPayload { url: string, method: string, headers: table<string, string>, body: string?, sign: boolean? }

local lib = require('lib-genvm')

---@class WEB
---@field allowed_tld { [string]: boolean }
---@field config table
---@field get_webdriver_session fun(ctx): string

---@type WEB
M.rs = __web; ---@diagnostic disable-line

M.allowed_schemas = {
	["http"] = true,
	["https"] = true,
}

local function table_has_val(tab, val)
	for _, v in ipairs(tab) do
		if v == val then
			return true
		end
	end
	return false
end

M.check_url = function(url)
	local split_url = lib.rs.split_url(url)

	if split_url == nil then
		lib.rs.user_error({
			causes = {"MALFORMED_URL"},
			fatal = false,
			ctx = {
				url = url
			}
		})
	end
	---@cast split_url -nil

	if not M.allowed_schemas[split_url.schema] then
		lib.rs.user_error({
			causes = {"SCHEMA_FORBIDDEN"},
			fatal = false,
			ctx = {
				schema = split_url.schema,
				url = url,
			}
		})
	end

	lib.log {
		always_allow_hosts = M.rs.config.always_allow_hosts,
		host = split_url.host,
	}

	if table_has_val(M.rs.config.always_allow_hosts, split_url.host) then
		return
	end

	if split_url.port ~= nil and split_url.port ~= 80 and split_url.port ~= 443 then
		lib.rs.user_error({
			causes = {"PORT_FORBIDDEN"},
			fatal = false,
			ctx = {
				port = split_url.port,
				url = url,
			}
		})
	end

	local from = split_url.host:find("[.]([^.]*)$")
	if from == nil then
		from = 0 -- not 1 for +1
	end
	local tld = string.sub(split_url.host, from + 1)

	lib.log{
		detected_tld = tld,
		host = split_url.host,
		from = from,
	}

	if not M.rs.allowed_tld[tld] then
		lib.rs.user_error({
			causes = {"TLD_FORBIDDEN"},
			fatal = false,
			ctx = {
				tld = tld,
				url = url,
			}
		})
	end
end

return M
