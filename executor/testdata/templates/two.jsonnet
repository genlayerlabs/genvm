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

			"message": import './message.json',

			"calldata": "{}"
		}
}
