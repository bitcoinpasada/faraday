# What else runs on this computer

## A computer inside the computer

Almost every Intel PC made since about 2008 has a second, small processor in its chipset: the Intel Management Engine, now called the Converged Security and Management Engine (CSME). It has its own memory and its own firmware, signed by Intel. It starts before the main processor and runs whenever the board has power, including when the computer looks switched off. AMD computers have the same kind of processor, the AMD Platform Security Processor (PSP). Intel Macs have the Management Engine too.

The Management Engine runs below the operating system. Faraday's kernel has no network stack and no drivers for Wi-Fi, Bluetooth or the computer's own disk, but none of that binds the Management Engine: it does not use Faraday's kernel.

## Remote management: AMT

Intel Active Management Technology (AMT) is a remote management feature built on the Management Engine. It is on business computers sold as vPro, with an Intel network adapter, and works only once someone has set it up. Consumer laptops have the Management Engine without AMT.

AMT lets whoever manages the computer see its screen and use its keyboard and mouse over the network (KVM redirection), whatever operating system runs, Faraday included. Set up one way, it shows a code on the screen that the person at the computer has to read out before a remote session starts; set up the other way, it need not ask. Faraday has no driver for the Management Engine, so it cannot tell whether AMT is set up or in use.

AMT reaches the network through the computer's Intel network adapters, wired or Wi-Fi. A computer with neither connected gives it no way out.

## What has been found

In 2009 Alexander Tereshkin and Rafal Wojtczuk of Invisible Things Lab showed at Black Hat USA that code could be made to run on the Management Engine's processor, below the operating system ("Ring −3").

In 2017 Intel published INTEL-SA-00075 (CVE-2017-5689): AMT firmware versions 6 to 11.6 let someone on the network take control of a computer where AMT was set up. Intel says it does not exist on consumer PCs with consumer firmware.

Later in 2017 Intel published INTEL-SA-00086: flaws in the Management Engine firmware, in versions from 6 to 11.20, that let someone run their own code on it. All but one need local access to the computer; the one that works over the network is in AMT and needs AMT's administrator access.

Since version 11, the Management Engine runs an operating system built from parts of MINIX 3, as Positive Technologies' analysis of its firmware showed. Positive Technologies also found a setting, tied to the NSA's High Assurance Platform program, that stops the Management Engine's main processes once the computer has started. It is an off switch, not a backdoor. The me_cleaner tool removes most of the Management Engine's firmware and can set this switch; writing the result usually takes an external flash programmer.

Code planted in firmware survives wiping the disk and installing a new operating system. There is no public evidence of a backdoor in use. What is documented is a closed, highly privileged component that has had serious flaws.

## A computer someone else manages

A work laptop managed by an employer may have AMT set up, and its IT department may be able to see the screen and type, whatever runs.

Do not use a computer someone else manages for keys that hold real money.

## If you are worried

Unplug the network cable before starting Faraday.

Consider taking out the Wi-Fi card, if the laptop has a removable one.

Check that Intel AMT, or Intel Manageability, is off in the firmware settings.

Power off fully after a session.

Or consider a dedicated laptop that does nothing but run Faraday and never goes online. Anything that sees a key on a computer that never goes online has nowhere to send it.

## A Raspberry Pi, and keys on several computers

A Raspberry Pi has no Management Engine and no PSP.

In a multisig wallet whose keys were made on devices from different makers, one compromised machine cannot move funds on its own.

## Where this comes from

Black Hat USA 2009: Alexander Tereshkin and Rafal Wojtczuk, "Introducing Ring -3 Rootkits", July 2009 (blackhat.com, archives). Intel: security advisory INTEL-SA-00075, CVE-2017-5689, released 1 May 2017; security advisory INTEL-SA-00086, released 20 November 2017; Intel AMT developer guide, "User Consent" (intel.com). Positive Technologies: Dmitry Sklyarov, Mark Ermolov and Maxim Goryachy, "Disabling Intel ME 11 via undocumented mode", August 2017 (ptsecurity.com); Dmitry Sklyarov, "Intel ME: The Way of the Static Analysis", TROOPERS17, March 2017. Andrew S. Tanenbaum, "An Open Letter to Intel", 2017 (cs.vu.nl). me_cleaner (github.com/corna/me_cleaner). All read 2026-10-24.
