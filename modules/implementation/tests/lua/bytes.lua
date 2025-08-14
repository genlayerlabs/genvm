local lib_genvm = require("lib-genvm")
local value2json = require("value2json")

function Test(ctx)
	return value2json(lib_genvm.rs.request(ctx, {
		method = "POST",
		url = "https://test-server.genlayer.com/body/echo",
		headers = {},
		body = "\xde\xad\xbe\xef",
	}))
end
