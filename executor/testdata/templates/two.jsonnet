{
	run(scriptfilefrom, scriptfileto)::
		{
			"vars": {
				"fromAddr": "AQAAAAAAAAAAAAAAAAAAAAAAAAA=",
				"toAddr": "AwAAAAAAAAAAAAAAAAAAAAAAAAA=",
			},
			"accounts": {
				"AQAAAAAAAAAAAAAAAAAAAAAAAAA=": {
					"code": scriptfilefrom
				},
				"AwAAAAAAAAAAAAAAAAAAAAAAAAA=": {
					"code": scriptfileto
				},
				"AgAAAAAAAAAAAAAAAAAAAAAAAAA=": {
					"code": null
				}
			},

			"message": {
				"contract_account": "AQAAAAAAAAAAAAAAAAAAAAAAAAA=",
				"sender_account": "AgAAAAAAAAAAAAAAAAAAAAAAAAA=",
				"value": null,
				"is_init": false,
			},

			"calldata": "{}"
		}
}
