local msg = import './message.json';
{
	run(scriptfilefrom, scriptfileto, cd)::
	{
		"vars": {
			"fromAddr": "AQAAAAAAAAAAAAAAAAAAAAAAAAA=",
			"toAddr": "AwAAAAAAAAAAAAAAAAAAAAAAAAA=",
		},
		"code": scriptfileto,

		"message": msg + {
			"contract_address": "AwAAAAAAAAAAAAAAAAAAAAAAAAA=",
			is_init: true,
		},

		"calldata": "{}",

		next: [
			{
				"vars": {
					"fromAddr": "AQAAAAAAAAAAAAAAAAAAAAAAAAA=",
					"toAddr": "AwAAAAAAAAAAAAAAAAAAAAAAAAA=",
				},
				"code": scriptfilefrom,

				"message": msg + {
					is_init: true,
				},

				"calldata": "{}",

				next: [
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

						"message": msg,

						"calldata": cd
					}
				]
			}
		]
	}
}
