# Screens and devices that can record

## A screen that records

A smart TV runs its own software on everything it shows, from every input, HDMI included. Many smart TVs run automatic content recognition: the TV samples what is on screen and sends it to its maker's servers to find out what is being watched.

In 2017 the US Federal Trade Commission and the State of New Jersey brought a case against the TV maker Vizio, which settled it. The complaint says Vizio's TVs captured information about a selection of the screen's pixels, second by second, from cable boxes, streaming devices, DVD players and other inputs, and sent it to Vizio's servers, where it was kept indefinitely. The software was on by default.

What such a TV keeps or sends is decided by its own software, which cannot be checked from the computer. A recovery phrase or a key shown on it may leave the room.

The Faraday app reads each screen's identity (its EDID) and names one that is a TV. On the stick, Faraday has no graphics driver and cannot read the screen's identity, so on a computer whose screen is a separate one it asks once whether the screen is a TV, and every start carries the line "On a TV? Stop: a smart TV can record its screen."

## A KVM switch or an IP-KVM

A KVM switch shares one keyboard, screen and mouse between computers. An IP-KVM does it over a network: it captures the computer's screen through HDMI, and appears to the computer as a USB keyboard, mouse and disk. PiKVM, one of them, describes itself as KVM over IP: full HD video from HDMI, an emulated USB keyboard and mouse, and access from a web browser or VNC.

So whoever can reach the IP-KVM over its network sees the screen and can type on the computer. This works without any help from the computer: the stick having no network does not stop it.

The Faraday app and the stick name one by its USB identity: the USB-gadget vendor ID that many such devices present with a keyboard and a disk, or a name that says KVM.

## A wireless keyboard or mouse

A wireless keyboard sends each key over the air to its receiver. Some send them unencrypted or poorly encrypted. In 2016 Bastille Networks disclosed MouseJack, in which wireless mice and keyboards from several makers on the 2.4 GHz band let someone in radio range type on the computer or read keystrokes (CERT/CC vulnerability note VU#981271), and KeySniffer, in which keyboards from eight makers sent every key unencrypted, readable from up to several hundred feet away.

The keys are in the air before they reach the computer, so the stick having no radio does not stop it. The Faraday app and the stick name a wireless receiver by its USB identity, and the stick warns before anything secret is typed.

## A virtual machine

A virtual machine is a computer inside another computer, its host. The host sees the virtual machine's memory, screen and keys, and can change any of them. Nothing running inside can tell what the host does.

The stick refuses to go on inside a virtual machine. The Faraday app on a virtual machine says the stick must start on the real computer, and that it cannot see the host's devices.

## Remote desktop, screen sharing, a wireless display

These send the picture to another device over a network. The stick has no network, so none of them can reach Faraday on the stick.

The Faraday app checks for them only when it is about to show the BitLocker recovery key, which is on the computer's own screen while Windows runs.

## Not a concern on the stick

Wi-Fi, Bluetooth and USB network adapters. The stick's kernel has no network support at all and no radio support: no driver binds to them, so they do nothing while Faraday runs, and Faraday does not warn about them.

USB video capture devices. The stick reads one only as a camera, for scanning: it gives Faraday what is plugged into its own input, not what Faraday shows, and nothing on the stick can send it anywhere. Faraday does not warn about them.

## What cannot be detected

A passive HDMI splitter sends the picture to a second screen or recorder and tells the computer nothing.

A keylogger inside a keyboard or its cable records the keys before they reach the computer and looks like an ordinary keyboard.

A camera pointed at the screen, or at the keyboard.

Look at what is between the computer and its screen and keyboard, and at the room.

## What to do

Use the laptop's own screen, or a plain monitor rather than a TV.

Use a wired keyboard, or the laptop's own.

Unplug what Faraday names before going on.

Start the stick on a real computer, never a virtual machine.

## Where this comes from

US Federal Trade Commission: complaint in FTC and State of New Jersey v. Vizio, Inc., case 2:17-cv-00758, District of New Jersey, filed 6 February 2017 (ftc.gov). PiKVM: its project description (github.com/pikvm/pikvm). Bastille Networks: KeySniffer (bastille.net, research). CERT/CC: vulnerability note VU#981271, public 23 February 2016 (kb.cert.org). All read 2026-10-12. What the stick's kernel has and lacks: OpenSigner's common kernel.forbidden (NET, WLAN, BT, DRM) and the Faraday image's common kernel.required (VIDEO_DEV, the camera the scanner opens), in Faraday's repository.
