# Lua API docs

## Format


---

## LLM

### exec_prompt_in_provider


```lua
fun(ctx: any, data: { prompt: Prompt, format: "bool"|"json"|"text", model: string, provider: string }):any
```

### providers


```lua
table<string, ProviderEntry>
```

### templates


```lua
{ eq_comparative: any, eq_non_comparative_leader: any, eq_non_comparative_validator: any }
```


---

## LLMExecPromptPayload

### images


```lua
userdata[]
```

### prompt


```lua
string
```

### response_format


```lua
"json"|"text"
```


---

## LLMExecPromptTemplatePayload

### [string]


```lua
string
```

### template


```lua
"EqComparative"|"EqNonComparativeLeader"|"EqNonComparativeValidator"
```


---

## MappedPrompt

### format


```lua
"bool"|"json"|"text"
```

### prompt


```lua
Prompt
```


---

## MergeStrategy


---

## ModelConfig

### enabled


```lua
boolean
```

### meta


```lua
any
```

### supports_image


```lua
boolean
```

### supports_json


```lua
boolean
```

### use_max_completion_tokens


```lua
boolean
```


---

## ModuleError

### causes


```lua
string[]
```

### ctx


```lua
table<string, any>
```

### fatal


```lua
boolean
```


---

## Prompt

### extra


```lua
table<string, any>|nil
```

### extra_merge_strategy


```lua
"merge_left"|"merge_right"|"none"|"replace"|table<string, "merge_left"|"merge_right"|"none"|"replace"|table<string, "merge_left"|"merge_right"|"none"|"replace">>...(+1)
```

### images


```lua
userdata[]
```

### max_tokens


```lua
integer
```

### seed


```lua
integer|nil
```

### system_message


```lua
string|nil
```

### temperature


```lua
number
```

### use_max_completion_tokens


```lua
boolean
```

### user_message


```lua
string
```


---

## ProviderEntry

### models


```lua
table<string, ModelConfig>
```


---

## ProvidersDB


---

## RS

### as_user_error


```lua
fun(val: any):ModuleError|nil
```

### base64_decode


```lua
fun(val: string):string
```

### base64_encode


```lua
fun(val: string):string
```

### filter_image


```lua
fun(image: string, filters: any[]):string
```

### filter_text


```lua
fun(text: string, filters: string[]):string
```

### json_parse


```lua
fun(val: string):any
```

### json_stringify


```lua
fun(val: any):string
```

### log_json


```lua
fun(val: any):nil
```

### random_bytes


```lua
fun(length: integer):string
```

### random_float


```lua
fun():number
```

### request


```lua
fun(ctx: any, req: { body: string|nil, url: string, headers: table<string, string>, method: string, error_on_status: boolean|nil, json: false|nil, response_body_max_size: integer|n...(too long)... string> }
```

### sleep_seconds


```lua
fun(duration: number):nil
```

### split_url


```lua
fun(url: string):{ schema: string, port: number|nil, host: string }|nil
```

### url_encode


```lua
fun(url: string):string
```

### user_error


```lua
fun(val: ModuleError):nil
```


---

## WEB

### allowed_tld


```lua
{ [string]: boolean }
```

### config


```lua
table
```

### get_webdriver_session


```lua
fun(ctx: any):string
```


---

## WebRenderPayload

### mode


```lua
"html"|"screenshot"|"text"
```

### size_limit


```lua
integer?
```

### url


```lua
string
```

### wait_after_loaded


```lua
number
```


---

## WebRequestPayload

### body


```lua
string?
```

### headers


```lua
table<string, string>
```

### method


```lua
"DELETE"|"GET"|"HEAD"|"OPTIONS"|"PATCH"...(+1)
```

### sign


```lua
boolean?
```

### size_limit


```lua
integer?
```

### url


```lua
string
```


---

## my_data


```lua
table
```


```lua
table
```
