{
	run(scriptfile)::
		{
			"vars": {},
			"runners": import './runners.jsonnet',
			"accounts": {
				"AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=": {
					"code": scriptfile
				},
				"AgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=": {
					"code": null
				}
			},

			"message": {
				"gas": 9007199254740991,
				"contract_account": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
				"sender_account": "AgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
				"value": null,
				"is_init": false,
			},

			"calldata": "{}"
		}
}
