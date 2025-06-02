local M = {}

---@alias WebRenderPayload { url: string, mode: "Text" | "HTML" | "Screenshot", wait_after_loaded: number }

local lib = require('lib-genvm')

---@class WEB
---@field allowed_tld { [string]: boolean }
---@field config table
---@field get_webdriver_session fun(ctx): string

---@type WEB
M.rs = __web; ---@diagnostic disable-line

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

	if split_url.host == "" then
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

	local from = split_url.host:find("\\.[a-z]$")
	if from == nil then
		from = 1
	end
	local tld = string.sub(split_url.host, from - 1)

	if M.rs.allowed_tld[tld] then
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
