local msg = import './message.json';
{
	run(scriptfile, method, args=[])::
		{
			"vars": {},
			"code": scriptfile,
			"message": msg + {
				"is_init": true,
			},
			"calldata": "{}",
			next: [
				{
					"vars": {},
					"code": null,
					"message": msg,
					"calldata": std.manifestJsonEx({
						"method": method,
						"args": args,
					}, "    "),
				},
			],
		}
}
