local M = {}

---@class WebRenderPayload
---@field url string
---@field mode "text" | "html" | "screenshot"
---@field wait_after_loaded number
---@field size_limit integer?

---@class WebRequestPayload
---@field url string
---@field method "GET" | "POST" | "HEAD" | "DELETE" | "OPTIONS" | "PATCH"
---@field headers table<string, string>
---@field body string?
---@field sign boolean?
---@field size_limit integer?

local lib = require("lib-genvm")

---@class WEB
---@field allowed_tld { [string]: boolean }
---@field config table
---@field get_webdriver_session fun(ctx): string

---@type WEB
M.rs = __web ---@diagnostic disable-line

--- Set of URL schemas permitted for web requests.
---@type table<string, boolean>
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

--- Validate a URL against allowed schemas, ports, and TLDs.
--- Raises a non-fatal `user_error` if any check fails (MALFORMED_URL, SCHEMA_FORBIDDEN, PORT_FORBIDDEN, TLD_FORBIDDEN).
--- URLs whose host is in `config.always_allow_hosts` bypass port and TLD checks.
---@param url string the URL to validate
M.check_url = function(url)
	local split_url = lib.rs.split_url(url)

	if split_url == nil then
		lib.rs.user_error {
			causes = { "MALFORMED_URL" },
			fatal = false,
			ctx = {
				url = url,
			},
		}
	end
	---@cast split_url -nil

	if not M.allowed_schemas[split_url.schema] then
		lib.rs.user_error {
			causes = { "SCHEMA_FORBIDDEN" },
			fatal = false,
			ctx = {
				schema = split_url.schema,
				url = url,
			},
		}
	end

	lib.log {
		level = "debug",
		message = "checking url",
		schema = split_url.schema,
		always_allow_hosts = M.rs.config.always_allow_hosts,
		host = split_url.host,
	}

	if table_has_val(M.rs.config.always_allow_hosts, split_url.host) then
		return
	end

	if split_url.port ~= nil and split_url.port ~= 80 and split_url.port ~= 443 then
		lib.rs.user_error {
			causes = { "PORT_FORBIDDEN" },
			fatal = false,
			ctx = {
				port = split_url.port,
				url = url,
			},
		}
	end

	local from = split_url.host:find("[.]([^.]*)$")
	if from == nil then
		from = 0 -- not 1 for +1
	end
	local tld = string.sub(split_url.host, from + 1)

	lib.log {
		level = "debug",
		message = "detected TLD",
		detected_tld = tld,
		host = split_url.host,
		from = from,
	}

	if not M.rs.allowed_tld[tld] then
		lib.rs.user_error {
			causes = { "TLD_FORBIDDEN" },
			fatal = false,
			ctx = {
				tld = tld,
				url = url,
			},
		}
	end
end

--- Split a resolved `addr` (as returned by `resolve_host`, i.e. an `ip:port` or
--- `[ipv6]:port` string) into the bare IP and whether it is IPv6.
---@param addr string
---@return string ip, boolean is_v6
local function strip_port(addr)
	local v6 = addr:match("^%[(.-)%]:%d+$") or addr:match("^%[(.-)%]$")
	if v6 ~= nil then
		return v6, true
	end

	local v4 = addr:match("^(.-):%d+$")
	if v4 ~= nil then
		return v4, false
	end

	return addr, addr:find(":") ~= nil
end

--- Whether an IPv4 address is not globally routable (loopback, private, etc.).
--- Returns nil if `ip` is not a valid dotted-quad.
---@param ip string
---@return boolean | nil
local function ipv4_is_bad(ip)
	local a, b, c, d = ip:match("^(%d+)%.(%d+)%.(%d+)%.(%d+)$")
	if a == nil then
		return nil
	end
	a, b, c, d = tonumber(a), tonumber(b), tonumber(c), tonumber(d)
	if a > 255 or b > 255 or c > 255 or d > 255 then
		return nil
	end

	if a == 0 then
		return true
	end -- "this" network 0.0.0.0/8
	if a == 10 then
		return true
	end -- private 10.0.0.0/8
	if a == 127 then
		return true
	end -- loopback 127.0.0.0/8
	if a == 169 and b == 254 then
		return true
	end -- link-local 169.254.0.0/16
	if a == 172 and b >= 16 and b <= 31 then
		return true
	end -- private 172.16.0.0/12
	if a == 192 and b == 168 then
		return true
	end -- private 192.168.0.0/16
	if a == 100 and b >= 64 and b <= 127 then
		return true
	end -- CGNAT 100.64.0.0/10
	if a >= 224 then
		return true
	end -- multicast/reserved/broadcast 224.0.0.0/3

	return false
end

--- Whether an IPv6 address is not globally routable (loopback, ULA, link-local, etc.).
---@param ip string
---@return boolean
local function ipv6_is_bad(ip)
	local lower = ip:lower()
	if lower == "::1" or lower == "::" then
		return true -- loopback / unspecified
	end

	local mapped = lower:match("^::ffff:(%d+%.%d+%.%d+%.%d+)$")
	if mapped ~= nil then
		return ipv4_is_bad(mapped) == true
	end

	local group = lower:match("^([%x]+)")
	if group == nil then
		group = 0 -- starts with "::"
	else
		group = tonumber(group, 16)
	end

	if group >= 0xfc00 and group <= 0xfdff then
		return true
	end -- unique-local fc00::/7
	if group >= 0xfe80 and group <= 0xfebf then
		return true
	end -- link-local fe80::/10
	if group >= 0xff00 then
		return true
	end -- multicast ff00::/8

	return false
end

--- Whether a resolved address is "bad" (not safe to connect to). Unparseable
--- addresses are treated as bad to fail closed.
---@param ip string
---@param is_v6 boolean
---@return boolean
local function addr_is_bad(ip, is_v6)
	if is_v6 then
		return ipv6_is_bad(ip)
	end
	return ipv4_is_bad(ip) ~= false
end

--- Resolve the host of `url` and pin it to a globally-routable address.
--- - if every resolved address is good, returns `url` unchanged;
--- - if some are bad but at least one is good, returns `url` with its host
---   replaced by a good address;
--- - if there is no good address, raises a non-fatal `user_error`.
---@param ctx any module context
---@param url string
---@return string
M.pin_url_to_good_host = function(ctx, url)
	local split_url = lib.rs.split_url(url)
	if split_url == nil then
		lib.rs.user_error {
			causes = { "MALFORMED_URL" },
			fatal = false,
			ctx = { url = url },
		}
	end
	---@cast split_url -nil

	local port = split_url.port
	if port == nil then
		port = split_url.schema == "https" and 443 or 80
	end

	local addrs = lib.rs.resolve_host(ctx, split_url.host .. ":" .. tostring(port))

	local good_ip = nil
	local good_is_v6 = false
	local any_bad = false
	for _, addr in ipairs(addrs) do
		local ip, is_v6 = strip_port(addr)
		if addr_is_bad(ip, is_v6) then
			any_bad = true
		elseif good_ip == nil then
			good_ip = ip
			good_is_v6 = is_v6
		end
	end

	if good_ip == nil then
		lib.rs.user_error {
			causes = { "ADDRESS_FORBIDDEN" },
			fatal = false,
			ctx = {
				url = url,
				host = split_url.host,
				addresses = addrs,
			},
		}
	end

	if not any_bad then
		return url
	end

	local host = good_is_v6 and ("[" .. good_ip .. "]") or good_ip
	return lib.rs.set_host(url, host)
end

return M
