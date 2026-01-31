extends Node

func _ready() -> void:
	var code = FileAccess.get_file_as_string("res://test.lua")
	$RblxVM.push_code(code)
	var textNode = $RblxVM.get_node("ConsoleInterface/ConsoleInterface/PanelContainer2/TextEdit")
	textNode.grab_focus()
	textNode.connect("text_submitted", Callable(self, "_on_text_submitted"))

func _on_text_submitted(text: String) -> void:
	var textEdit = $RblxVM.get_node("ConsoleInterface/ConsoleInterface/PanelContainer2/TextEdit")
	var logWindow = $RblxVM.get_node("ConsoleInterface/ConsoleInterface/PanelContainer/RichTextLabel")
	textEdit.text = ""
	
	var args = text.split(" ", false)
	
	match args[0].to_lower():
		"?": logWindow.add_text("'?' - Show detailed list of all commands available.\n'clear' - Clear the console.\n'autoscroll <true/false/on/off>' - Toggle autoscrolling of the console.\n\n'lua \"<filename>\"' - Run a Luau file.\n")
		"clear": logWindow.text = ""
		"autoscroll":
			if args.size() == 1:
				logWindow.add_text("Expected argument <true/false/on/off>\n")
				return
			match args[1].to_lower():
				"true","on": 
					logWindow.scroll_following = true
					logWindow.add_text("Autoscroll set to true\n")
				"false","off":
					logWindow.scroll_following = false
					logWindow.add_text("Autoscroll set to false\n")
				_: logWindow.add_text("Expected argument <true/false/on/off>\n")
		"lua":
			if args.size() == 1:
				logWindow.add_text("Expected argument\n")
				return
			print(args[1])
			if !FileAccess.file_exists(args[1]):
				logWindow.add_text("Expected path to existing file\n")
				return
			var code = FileAccess.get_file_as_string(args[1])
			$RblxVM.push_code(code)
		_: logWindow.add_text("Unknown command\n")
