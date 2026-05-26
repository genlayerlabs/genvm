---@class Rat
---@operator add(Rat|number): Rat
---@operator sub(Rat|number): Rat
---@operator mul(Rat|number): Rat
---@operator div(Rat|number): Rat
---@operator mod(Rat|number): Rat
---@operator unm: Rat
local Rat = {}

---@return string numerator as a decimal string
function Rat:numer() end

---@return string denominator as a decimal string
function Rat:denom() end

---@return number
function Rat:to_float() end

---@return Rat
function Rat:abs() end

---@return integer -1, 0, or 1
function Rat:sign() end

---@return boolean
function Rat:is_zero() end

---@return boolean
function Rat:is_integer() end

---@return Rat
function Rat:floor() end

---@return Rat
function Rat:ceil() end

---@return Rat
function Rat:trunc() end

---@param exp integer
---@return Rat
function Rat:pow(exp) end

---@class RatLib
---@field new fun(value: string|number|Rat, denom: integer|string|nil): Rat
---@field is_rat fun(value: any): boolean

---@type RatLib
rat = rat ---@diagnostic disable-line
