local script = Instance.new("Script")
local module_script = Instance.new("ModuleScript")
script.Parent = game
script.Source = [[
	local function test(script, game)
		print("hiya from rblx-godot! :3", _VERSION)
		print("script action:", script, game)
		warn("meow :3")
	end
	task.delay(3, print, "3 seconds later :3")
	test(script, game)
	print(task.wait(5), " seconds later!! :3")

	local module = require(script.ModuleScript)
	print(module)
	module()
	script:Destroy()
]]

module_script.Parent = script
module_script.Source = [[
local module = {}
print("hiiii from module script :3")
local f = function()
	warn("meeeowwww :3")
end
return f
]]

script.RunContext = Enums.RunContext.Server
print(script.Enabled)
script.Enabled = true
print("user initiated action:", script, game)
