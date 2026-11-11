# Windows command execution

Use `shell: "cmd.exe"` and `login: false` with `exec_command` in this workspace.
The default PowerShell resolves to the Microsoft Store `WindowsApps/pwsh.exe`
entry point and fails with `CreateProcessAsUserW` error 5 in the agent sandbox.
Explicit `cmd.exe` has been verified to start Cargo, Rust, and the Tauri CLI
under the existing sandbox permissions.

Use cmd syntax for these commands. Prefer `npm.cmd` when running npm scripts.
