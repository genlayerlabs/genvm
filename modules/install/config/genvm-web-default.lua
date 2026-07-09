local lib = require("lib-genvm")
local web = require("lib-web")

local function status_is_good(status)
	return status >= 200 and status < 300 or status == 304
end

function Render(ctx, payload)
	---@cast payload WebRenderPayload
	web.check_url(payload.url)

	local url_params = "?url="
		.. lib.rs.url_encode(payload.url)
		.. "&mode="
		.. payload.mode
		.. "&waitAfterLoaded="
		.. tostring(payload.wait_after_loaded or 0)

	-- The webdriver is operator-configured infrastructure, commonly on a
	-- private address (docker compose networks resolve it to RFC1918), so this
	-- transport hop must not go through the SSRF-filtered client. The
	-- contract-controlled input is the target URL, validated by `check_url`
	-- above; the webdriver host is trusted the same way LLM backends are.
	local result = lib.rs.request(ctx, {
		method = "GET",
		url = web.rs.config.webdriver_host .. "/render" .. url_params,
		headers = {},
		error_on_status = true,
		response_body_max_size = payload.size_limit,
		unfiltered = true,
	})

	lib.log {
		level = "debug",
		message = "web render result",
		result = result,
	}

	local status = tonumber(result.headers["resulting-status"])

	if not status_is_good(status) then
		lib.rs.user_error {
			causes = { "WEBPAGE_LOAD_FAILED" },
			fatal = false,
			ctx = {
				url = payload.url,
				status = status,
				body = result.body,
			},
		}
	end

	if payload.mode == "screenshot" then
		return {
			image = result.body,
		}
	else
		return {
			text = result.body,
		}
	end
end

function Request(ctx, payload)
	---@cast payload WebRequestPayload

	-- `check_url` returns true when the host is in `always_allow_hosts`; such
	-- hosts are sent through the unfiltered client. Everything else goes through
	-- the SSRF-guarded client, whose resolver drops non-globally-routable
	-- addresses. The hostname is kept in the URL either way (rewriting it to an
	-- IP would break TLS SNI / cert verification and is what broke HTTPS).
	local allowlisted = web.check_url(payload.url)

	local success, result = pcall(lib.rs.request, ctx, {
		method = payload.method,
		url = payload.url,
		headers = payload.headers,
		body = payload.body,
		sign = payload.sign,
		response_body_max_size = payload.size_limit,
		unfiltered = allowlisted,
	})

	if success then
		return result
	end

	lib.reraise_with_fatality(result, false)
end
