# falcon-bms-control-server, the next version

The next version of the server component of the `falcon-bms-control` app.

## How to build this?

This will only build for windows (probably `x86_64-pc-windows-gnu` and certainly `x86_64-pc-windows-msvc`, since it directly uses Win32 API and there is no point in building it for other platforms. After all BMS runs on windows (or wine) only. 

- Install MSYS2 MINGW64
- Install the required packages:
	- rustup / cargo
	- nasm
	- mingw-w64-clang-x86_64-clang
	- cmake

That should be all that is needed for a succefull incantation of `cargo build`.

