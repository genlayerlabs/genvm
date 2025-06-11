local lib_genvm = require("lib-genvm")
local value2json = require("value2json")

function Test(ctx)
	return value2json(lib_genvm.rs.request(ctx, {
		method = "GET",
		url = "http://localhost:4445/body",
		headers = {},
	}))
end
