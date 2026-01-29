local msg = import './message.json';
{
	run(scriptfile)::
		{
			"vars": {},
			"code": scriptfile,
			"message": msg + {
				"is_init": true,
			},
			"calldata": "{}",
		}
}
