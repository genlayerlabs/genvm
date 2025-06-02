local lib = require('lib-genvm')
local web = require('lib-web')

local function render_screenshot(ctx)
	local success, result = pcall(function()
		return lib.rs.request(ctx, {
		method = 'GET',
		url = web.rs.config.webdriver_host .. '/session/' .. ctx.session .. '/execute/sync',
		headers = {},
		error_on_status = true,
	}) end)

	if not success then
		lib.reraise_with_fatality(result, true)
	end
	---@cast result -unknown

	return {
		image = lib.rs.base64_decode(lib.rs.json_parse(result.body).value)
	}
end

local function render_impl(ctx, payload)
	---@cast payload WebRenderPayload

	web.check_url(payload.url)

	if ctx.session == nil then
		ctx.session = web.get_webdriver_session(ctx)
	end


	lib.rs.request(ctx, {
		method = 'POST',
		url = web.rs.config.webdriver_host .. '/session/' .. ctx.session .. '/url',
		headers = {
			['Content-Type'] = 'application/json; charset=utf-8',
		},
		body = lib.rs.json_stringify({
			url = payload.url
		})
	})

	if payload.wait_after_loaded > 0 then
		lib.rs.sleep_seconds(payload.wait_after_loaded)
	end

	if payload.mode == "Screenshot" then
		return render_screenshot(ctx)
	end

	local script
	if payload.mode == "HTML" then
		script = '{ "script": "return document.body.innerHTML", "args": [] }'
	else
		script = '{ "script": "return document.body.innerText.replace(/[\\s\\n]+/g, \\" \\")", "args": [] }'
	end

	local success, result = pcall(function()
		return lib.rs.request(ctx, {
		method = 'POST',
		url = web.rs.config.webdriver_host .. '/session/' .. ctx.session .. '/execute/sync',
		headers = {
			['Content-Type'] = 'application/json; charset=utf-8',
		},
		body = script,
		error_on_status = true,
	}) end)

	if not success then
		lib.reraise_with_fatality(result, true)
	end
	---@cast result -unknown

	local result = lib.rs.json_parse(result.body)
	return {
		text = result.value,
	}
end

function render(ctx, payload)
	---@cast payload WebRenderPayload
	local success, result = pcall(render_impl, ctx, payload)

	if success then
		return result
	end

	lib.reraise_with_fatality(result, true)
end

function request(ctx, payload)
	error("todo")
end
