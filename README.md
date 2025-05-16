> [!WARNING]
> This project is still heavily in development and as such you will see very frequent changes in the codebase and crashes from unimplemented features.

# rblx-godot

![image](https://cdn.discordapp.com/attachments/1284545242229641392/1373021365606809680/image-113.png?ex=6828e54b&is=682793cb&hm=ef97913067397c7a00905aac2ec0de2267f200e3e958240f49525c7c456ac3b2&)

A GDExtension written in Rust that adds [Luau](https://luau-lang.org) and creates a `RblxVM` class for Godot to create a Roblox-compatible environment for games to run in.
*(+ some extras)*

About
-----
A stub for rbxl games to run insde Godot, written completely inside Rust leveraging the low-level API of Godot Game Engine.

Features
--------
- Implementation of a Roblox VM that runs Luau and the task scheduler as needed.
- TODO: Implementation of Instances, Roblox data types
- Implementation of Actors
- Implementation of scripts
- TODO: Implementation of UI
- TODO: Implementation of inputs
- TODO: Implementation of rendering
- TODO: Implementation of loading .rbxl files
- TODO: Implementation of physics
- TODO: Implementation of networking

Compiling
------------
- Clone the repo
- Install rust nightly
- Run `cargo build`
- [A test project is included in the repo](https://github.com/roblox-to-godot-project/roblox-to-godot-project/tree/master/godot)

**Special thanks**
------
- https://godotengine.org/
- https://github.com/WeaselGames/godot_luaAPI (now archived, rest in peace...)
- https://github.com/godot-rust/gdext
- https://github.com/mlua-rs/mlua
- https://github.com/luau-lang/luau
