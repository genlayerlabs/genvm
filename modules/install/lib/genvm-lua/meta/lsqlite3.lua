---@meta

---@class lsqlite3
---@field OK integer
---@field ERROR integer
---@field INTERNAL integer
---@field PERM integer
---@field ABORT integer
---@field BUSY integer
---@field LOCKED integer
---@field NOMEM integer
---@field READONLY integer
---@field INTERRUPT integer
---@field IOERR integer
---@field CORRUPT integer
---@field NOTFOUND integer
---@field FULL integer
---@field CANTOPEN integer
---@field PROTOCOL integer
---@field EMPTY integer
---@field SCHEMA integer
---@field TOOBIG integer
---@field CONSTRAINT integer
---@field MISMATCH integer
---@field MISUSE integer
---@field NOLFS integer
---@field FORMAT integer
---@field RANGE integer
---@field NOTADB integer
---@field ROW integer
---@field DONE integer
---@field INTEGER integer
---@field FLOAT integer
---@field TEXT integer
---@field BLOB integer
---@field NULL integer
---@field CREATE_INDEX integer
---@field CREATE_TABLE integer
---@field CREATE_TEMP_INDEX integer
---@field CREATE_TEMP_TABLE integer
---@field CREATE_TEMP_TRIGGER integer
---@field CREATE_TEMP_VIEW integer
---@field CREATE_TRIGGER integer
---@field CREATE_VIEW integer
---@field DELETE integer
---@field DROP_INDEX integer
---@field DROP_TABLE integer
---@field DROP_TEMP_INDEX integer
---@field DROP_TEMP_TABLE integer
---@field DROP_TEMP_TRIGGER integer
---@field DROP_TEMP_VIEW integer
---@field DROP_TRIGGER integer
---@field DROP_VIEW integer
---@field INSERT integer
---@field PRAGMA integer
---@field READ integer
---@field SELECT integer
---@field TRANSACTION integer
---@field UPDATE integer
---@field ATTACH integer
---@field DETACH integer
---@field ALTER_TABLE integer
---@field REINDEX integer
---@field ANALYZE integer
---@field CREATE_VTABLE integer
---@field DROP_VTABLE integer
---@field FUNCTION integer
---@field DENY integer
---@field IGNORE integer
---@field open fun(filename: string): sqlite3_db
---@field open_memory fun(): sqlite3_db
---@field version fun(): string
---@field lversion fun(): string
---@field complete fun(sql: string): boolean
---@field temp_directory fun(directory: string)
local lsqlite3 = {}

---@class sqlite3_db
local sqlite3_db = {}

---@param sql string
---@param func? fun(udata: any, ncols: integer, values: string[], names: string[]): integer
---@param udata? any
---@return integer result_code
function sqlite3_db:exec(sql, func, udata) end

---@param sql string
---@return sqlite3_stmt?
function sqlite3_db:prepare(sql) end

---@return integer
function sqlite3_db:close() end

---@return integer
function sqlite3_db:close_vm() end

---@return boolean
function sqlite3_db:isopen() end

---@return integer
function sqlite3_db:errcode() end

---@return string
function sqlite3_db:errmsg() end

---@return integer
function sqlite3_db:changes() end

---@return integer
function sqlite3_db:total_changes() end

---@return integer
function sqlite3_db:last_insert_rowid() end

---@param sql string
---@return fun(): table<string, any>
function sqlite3_db:nrows(sql) end

---@param sql string
---@return fun(): any ...
function sqlite3_db:rows(sql) end

---@param sql string
---@return fun(): any ...
function sqlite3_db:urows(sql) end

---@param func fun(udata: any, event: integer, dbname: string, tblname: string, rowid: integer)
---@param udata? any
function sqlite3_db:update_hook(func, udata) end

---@param func fun(udata: any): integer
---@param udata? any
function sqlite3_db:commit_hook(func, udata) end

---@param func fun(udata: any)
---@param udata? any
function sqlite3_db:rollback_hook(func, udata) end

---@param func fun(udata: any, action: integer, dbname: string, tblname: string): integer
---@param udata? any
function sqlite3_db:set_authorizer(func, udata) end

---@param n integer
---@param func fun(udata: any): integer
---@param udata? any
function sqlite3_db:progress_handler(n, func, udata) end

---@param func fun(udata: any): integer
---@param udata? any
function sqlite3_db:busy_handler(func, udata) end

---@param t integer
function sqlite3_db:busy_timeout(t) end

---@param name string
---@param nargs integer
---@param func fun(ctx: sqlite3_context, ...: any)
---@param step? fun(ctx: sqlite3_context, ...: any)
---@param final_? fun(ctx: sqlite3_context)
function sqlite3_db:create_function(name, nargs, func, step, final_) end

---@param name string
---@param nargs integer
---@param step fun(ctx: sqlite3_context, ...: any)
---@param final_ fun(ctx: sqlite3_context)
function sqlite3_db:create_aggregate(name, nargs, step, final_) end

---@param name string
---@param func fun(s1: string, s2: string): integer
function sqlite3_db:create_collation(name, func) end

---@param sql string
---@param udata? any
function sqlite3_db:trace(sql, udata) end

function sqlite3_db:interrupt() end

---@param name string
---@return string
function sqlite3_db:db_filename(name) end

---@param ext_name string
function sqlite3_db:load_extension(ext_name) end

---@class sqlite3_stmt
local sqlite3_stmt = {}

---@return integer result_code
function sqlite3_stmt:step() end

---@return integer result_code
function sqlite3_stmt:reset() end

---@return integer result_code
function sqlite3_stmt:finalize() end

---@return boolean
function sqlite3_stmt:isopen() end

---@return integer
function sqlite3_stmt:columns() end

---@param n integer
---@param value any
---@return integer result_code
function sqlite3_stmt:bind(n, value) end

---@param n integer
---@param blob string
---@return integer result_code
function sqlite3_stmt:bind_blob(n, blob) end

---@param tbl table<string, any>
---@return integer result_code
function sqlite3_stmt:bind_names(tbl) end

---@param ... any
---@return integer result_code
function sqlite3_stmt:bind_values(...) end

---@return integer
function sqlite3_stmt:bind_parameter_count() end

---@param n integer
---@return string
function sqlite3_stmt:bind_parameter_name(n) end

---@param n integer
---@return any
function sqlite3_stmt:get_value(n) end

---@return any ...
function sqlite3_stmt:get_values() end

---@return any ...
function sqlite3_stmt:get_uvalues() end

---@param n integer
---@return string
function sqlite3_stmt:get_name(n) end

---@return string ...
function sqlite3_stmt:get_names() end

---@return string ...
function sqlite3_stmt:get_unames() end

---@param n integer
---@return string
function sqlite3_stmt:get_type(n) end

---@return string ...
function sqlite3_stmt:get_types() end

---@return string ...
function sqlite3_stmt:get_utypes() end

---@return table<string, string>
function sqlite3_stmt:get_named_types() end

---@return table<string, any>
function sqlite3_stmt:get_named_values() end

---@return fun(): table<string, any>
function sqlite3_stmt:nrows() end

---@return fun(): any ...
function sqlite3_stmt:rows() end

---@return fun(): any ...
function sqlite3_stmt:urows() end

---@return integer
function sqlite3_stmt:last_insert_rowid() end

---@class sqlite3_context
local sqlite3_context = {}

---@return any
function sqlite3_context:user_data() end

---@return integer
function sqlite3_context:aggregate_count() end

---@return any
function sqlite3_context:get_aggregate_data() end

---@param udata any
function sqlite3_context:set_aggregate_data(udata) end

---@param value any
function sqlite3_context:result(value) end

function sqlite3_context:result_null() end

---@param n number
function sqlite3_context:result_number(n) end

---@param n integer
function sqlite3_context:result_int(n) end

---@param s string
function sqlite3_context:result_text(s) end

---@param b string
function sqlite3_context:result_blob(b) end

---@param err string
function sqlite3_context:result_error(err) end

return lsqlite3
