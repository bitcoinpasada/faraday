# windows-sys (the Starter on Windows)

**Purpose.** The Windows Starter lists disks, writes them, installs
itself per user, reads this computer's facts and restarts into the
firmware (`docs/STARTER.md` §2, §4.3, §5.2, §7). The Win32 calls std does
not make go through `windows-sys`: the IOCTLs on disks and volumes
(`DeviceIoControl`), the volume enumeration (`FindFirstVolumeW`,
`GetVolumeInformationW`, `GetVolumePathNamesForVolumeNameW`,
`QueryDosDeviceW`), the registry (`RegGetValueW` for HKLM's facts,
`RegCreateKeyExW`, `RegSetValueExW`, `RegDeleteTreeW` and
`RegOpenKeyExW` for the HKCU uninstall entry), `GetFirmwareType`,
`IsWow64Process2`, the elevation (`ShellExecuteExW` with `runas`), the
writer's pipe (`CreateNamedPipeW` with a DACL from
`ConvertStringSecurityDescriptorToSecurityDescriptorW`,
`ConnectNamedPipe`, `GetNamedPipeClientProcessId`), the account's SID
(`OpenProcessToken`, `GetTokenInformation`, `ConvertSidToStringSidW`),
known folders (`SHGetKnownFolderPath`) and the waits on a started
process. `faraday/shells/starter/src/windows/sys.rs` is the only code that
calls them, and every `unsafe` block is there, each with its `SAFETY`
note; what the answers mean is read in modules with no Win32 call in
them, tested on Linux with recorded answers.

**Why this crate.** It is already in the build: winit 0.30 depends on
`windows-sys` 0.52.0 on Windows. The Starter names the same release,
pinned (`=0.52.0`), with the features its calls need, so `Cargo.lock`
gains no new crate, only the edge from `faraday-starter-shell` to it. It is
Microsoft's generated declarations of the Win32 API and nothing else:
no code runs from it beyond the calls made. MIT or Apache-2.0, which
`deny.toml` allows. STARTER.md §2 rules out any other Windows crate.

Not used from it: COM interfaces (it declares none). The Start menu
shortcut is written as a Shell Link file's bytes by
`windows/shortcut.rs` instead of through `IShellLinkW`, so no vtable is
declared by hand.

**Review.** Trusted by publisher (kennykerr) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.