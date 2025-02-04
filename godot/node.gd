extends Node

func _ready() -> void:
	var code = FileAccess.get_file_as_string("res://test.lua")
	$RblxVM.push_code(code)
