# Run once at the first logon of the test virtual machine, from the answer
# disk (label WINVM): BitLocker on C: with a TPM protector and a numerical
# recovery password, the way a person turns it on in Windows 11 Pro. The
# recovery password and the status are written to the answer disk, for the
# test machine only. Fast startup is turned off so that a shutdown is a
# shutdown. The installation DVD is ejected first: with a bootable disc in
# the drive manage-bde refuses the TPM protector. Windows may already have
# started encrypting by itself (device encryption, without a protector);
# the commands below then only add the protectors.
$disk = (Get-Volume | Where-Object { $_.FileSystemLabel -eq 'WINVM' -and $_.DriveLetter }).DriveLetter
$out = "${disk}:\out"
New-Item -ItemType Directory -Force $out | Out-Null
Start-Transcript "$out\firstlogon.log"
powercfg /h off
$shell = New-Object -ComObject Shell.Application
Get-CimInstance Win32_CDROMDrive | ForEach-Object {
	$shell.Namespace(17).ParseName($_.Drive).InvokeVerb('Eject')
}
Start-Sleep 5
manage-bde -protectors -add C: -TPM
manage-bde -protectors -add C: -RecoveryPassword
manage-bde -on C: -UsedSpaceOnly -SkipHardwareTest
manage-bde -protectors -get C: | Out-File -Encoding ascii "$out\recovery.txt"
do {
	Start-Sleep 10
	$v = Get-BitLockerVolume -MountPoint C:
	"$(Get-Date -Format s) $($v.VolumeStatus) $($v.EncryptionPercentage)%" | Out-File -Append -Encoding ascii "$out\progress.txt"
} while ($v.VolumeStatus -ne 'FullyEncrypted')
manage-bde -status C: | Out-File -Encoding ascii "$out\status.txt"
manage-bde -protectors -get C: | Out-File -Encoding ascii "$out\protectors.txt"
'done' | Out-File -Encoding ascii "$out\done.txt"
Stop-Transcript
Start-Process cmd -ArgumentList '/k', 'manage-bde -status C: & manage-bde -protectors -get C: -Type TPM'
