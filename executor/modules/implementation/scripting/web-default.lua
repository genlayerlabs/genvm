local lib = require('lib-genvm')
local web = require('lib-web')

local function render_screenshot(ctx)
	local result = lib.rs.request(ctx, {
		method = 'GET',
		url = web.rs.config.webdriver_host .. '/session/' .. ctx.session .. '/screenshot',
		headers = {},
		error_on_status = true,
		json = true,
	})

	return {
		image = lib.rs.base64_decode(result.body.value)
	}
end

local function render_impl(ctx, payload)
	---@cast payload WebRenderPayload
	web.check_url(payload.url)

	if ctx.session == nil then
		ctx.session = web.rs.get_webdriver_session(ctx)
	end


	local url_request = lib.rs.request(ctx, {
		method = 'POST',
		url = web.rs.config.webdriver_host .. '/session/' .. ctx.session .. '/url',
		headers = {
			['Content-Type'] = 'application/json; charset=utf-8',
		},
		body = lib.rs.json_stringify({
			url = payload.url
		}),
	})

	if url_request.status ~= 200 then
		lib.rs.user_error({
			causes = {"WEBPAGE_LOAD_FAILED"},
			fatal = false,
			ctx = {
				url = payload.url,
				status = url_request.status,
				body = url_request.body,
			}
		})
	end

	if payload.wait_after_loaded > 0 then
		lib.rs.sleep_seconds(payload.wait_after_loaded)
	end

	if payload.mode == "screenshot" then
		return render_screenshot(ctx)
	end

	local script
	if payload.mode == "html" then
		script = '{ "script": "return document.body.innerHTML.trim()", "args": [] }'
	else
		script = '{ "script": "return document.body.innerText.replace(/[\\\\s\\\\n]+/g, \\" \\").trim()", "args": [] }'
	end

	local result = lib.rs.request(ctx, {
		method = 'POST',
		url = web.rs.config.webdriver_host .. '/session/' .. ctx.session .. '/execute/sync',
		headers = {
			['Content-Type'] = 'application/json; charset=utf-8',
		},
		body = script,
		json = true,
		error_on_status = true,
	})

	return {
		text = result.body.value,
	}
end

function render(ctx, payload)
	---@cast payload WebRenderPayload
	return render_impl(ctx, payload)
end

function request(ctx, payload)
	error("todo")
end
