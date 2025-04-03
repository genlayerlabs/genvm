function dump(o)
    if type(o) == 'table' then
       local s = '{ '
       for k,v in pairs(o) do
          if type(k) ~= 'number' then k = '"'..k..'"' end
          s = s .. '['..k..'] = ' .. dump(v) .. ', '
       end
       return s .. '} '
    else
       return tostring(o)
    end
 end

print("Lua loaded!")
print(dump(greyboxing.available_backends))

function exec_prompt(args)
    print("WOW A PROMPT", dump(args))
    local handler = args.handler;
    local prompt = args.prompt;
    return handler:exec_in_backend({provider = 'openai', text = prompt})
end
