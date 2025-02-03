local script = Instance.new("Script")
script.Parent = game
script.Source = [[
	local function test(script, game)
		print("hiya from roblox-to-godot! :3", _VERSION)
		print("script action:", script, game)
		warn("meow :3")
	end
	task.delay(3, print, "3 seconds later :3")
	test(script, game)
	print(task.wait(5), " seconds later!! :3")
	script:Destroy()
]]
--script.RunContext = Enums.RunContext.Server
print(script.Enabled)
script.Enabled = true
print("user initiated action:", script, game)