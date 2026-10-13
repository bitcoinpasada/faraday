# Starting an Intel Mac from the stick

## Which Macs

A Mac with an Intel processor and 64-bit EFI firmware starts the Faraday stick. Faraday Starter names this Mac and its model identifier on Home; the identifier is also in System Information, under Hardware Overview.

Some of the first Intel Macs, from 2006 and 2007, have 32-bit EFI firmware and cannot start it. In Terminal, ioreg -l -p IODeviceTree | grep firmware-abi prints EFI32 on such a Mac and EFI64 on the others.

A Mac with Apple silicon (M1 and later) cannot start Faraday yet.

## Macs with the T2 chip

These Macs have the Apple T2 Security Chip: MacBook Pro and MacBook Air introduced from 2018 to 2020 with an Intel processor, iMac (Retina 5K, 27-inch, 2020), iMac Pro, Mac mini (2018) and Mac Pro (2019). Apple's list is "Mac computers that have the Apple T2 Security Chip".

On these Macs, Startup Security Utility allows starting from the stick. Lower it before the first start, and set it back afterwards. A Mac without the T2 chip has no such setting: skip both steps.

## Lower Startup Security (T2 only)

Turn the Mac on and hold Command-R until the Apple logo appears, to start macOS Recovery. Choose Utilities, then Startup Security Utility, and enter the password of an administrator account on this Mac.

Set Secure Boot to No Security. Medium Security is not enough: it starts only what Apple or Microsoft signed. Set External Boot to Allow booting from external or removable media. Quit, then shut down from the Apple menu.

## Start from the stick

Plug the stick in with the Mac shut down. Press the power button and hold the Option key (Alt) at once, until Startup Manager shows the disks it can start. Choose EFI Boot, the stick, and press Return.

If a firmware password is set, the Mac asks for it before Startup Manager. Without the password, the Mac does not start from the stick: whoever set the password can turn it off in macOS Recovery, under Utilities, Startup Security Utility or Firmware Password Utility.

## Keyboard and mouse

The built-in keyboard works in Faraday on a Mac laptop from 2011 to 2015, except the MacBook (Retina, 12-inch). The built-in trackpad does not work on any Mac.

On the MacBook (Retina, 12-inch), the MacBook Pro from 2016 and 2017, and every Mac with the T2 chip, the built-in keyboard does not work in Faraday: plug in a USB keyboard. On an iMac, Mac mini or Mac Pro, use a USB keyboard and mouse.

On a Mac from before 2011, Faraday may not find the built-in keyboard, or a keyboard plugged straight into the Mac. A USB keyboard plugged into a USB 2.0 hub, and the hub into the Mac, is found.

A USB keyboard or mouse is accepted only after the approval code Faraday shows is typed on it, as on any computer.

On a Mac keyboard with the ISO layout, which has a narrow key left of Z, that key and the key below Esc may be swapped in Faraday.

## Afterwards: Startup Security back (T2 only)

Turn the Mac off from Faraday and pull the stick out. Start macOS Recovery with Command-R again, open Startup Security Utility, and set Secure Boot back to Full Security and External Boot back to Disallow booting from external or removable media.

## Where this comes from

Apple: "Mac startup key combinations" (support.apple.com/102603), "About Startup Security Utility on a Mac with the Apple T2 Security Chip" (support.apple.com/102522), "Mac computers that have the Apple T2 Security Chip" (support.apple.com/103265), "Set a firmware password on your Mac" (support.apple.com/102384), and the "Identify your Mac model" pages (support.apple.com/108052, 102869, 103257, 108054, 102852, 102887). The Linux kernel's applespi driver (drivers/input/keyboard/applespi.c) for the built-in keyboards of 2015 to 2017. All read 2026-10-13.
