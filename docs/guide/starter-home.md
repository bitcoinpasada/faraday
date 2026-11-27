# Starter: Home

**Kind:** Reference
**Status:** Checked against the app
**Starter page:** `Home`
**Key:** starter-home

Home shows this computer, the checklist for starting it from a Faraday stick and the cards that apply to it. The button on the This computer card is the next step.

## This computer

**Key:** starter-home.computer

The card names this computer, or on a Mac its model, and says whether it can start Faraday. Its button is the first of the following that applies.

**Make a Faraday stick** is shown when no stick is attached or when this computer cannot start one. **Restart into the BIOS** is shown while Secure Boot is on or unknown, and **Restart from the stick** while it is off. **Start this Mac from the stick** opens the start steps on a Mac. **I have its rescue disk or password** is shown first where another encryption product guards a drive. A restart is held when a TV, a KVM switch or a virtual machine is found.

## Checks

**Key:** starter-home.checks

One line above the checks says whether this computer can start Faraday, can start it without a camera, or cannot. Below it each check is green, amber or red: **Start mode**, **Secure Boot** and **Camera** always, and **Processor**, a virtual machine, a TV, a KVM switch and **BitLocker** only when something is wrong.

**Start mode** reads **UEFI**, or **Legacy BIOS: not compatible** in red. The stick starts in UEFI mode only: turn UEFI on in the BIOS, where this computer's maker page in the Guide shows how, or use another computer.

**Secure Boot** reads **Off**, or **On** in amber. **On** is a step to take, not a failure: press **Restart into the BIOS** and turn Secure Boot off there.

## Camera, processor and the rest

**Key:** starter-home.checks-other

**Camera** reads green when the Starter expects the camera to work on the stick: a USB camera that offers uncompressed frames. Amber means none was found, it is not usable on the stick, or it could not be checked. Faraday still starts, and files go by stick instead of the camera. A camera on a ribbon cable (MIPI) does not work.

**Processor** is shown only for ARM, in red: this computer cannot start Faraday, so use another. A virtual machine, a TV or a KVM switch is shown in red and the restart is held: start the stick on the real computer with its own screen. On Windows, BitLocker On is amber: press **Temporarily suspend BitLocker** before the restart.

## Checklist

**Key:** starter-home.checklist

Each row opens its page. **Make a Faraday stick** shows **No Faraday stick attached** or the attached stick's version, and opens the disks. **Start this computer from the stick** (**Start this Mac from the stick** on a Mac) shows the Secure Boot state and opens the start steps. **Turn Secure Boot back on** is shown only while Secure Boot is off and opens the same steps.

The next step is marked: it is the first of the stick and the start that is not done.

## Faraday Demo

**Key:** starter-home.desktop

**Open Faraday Demo** opens Faraday Demo inside the Starter, with the version shown on the card.

## Update

**Key:** starter-home.update

This card is shown as **Update the installed Faraday** while the installed Starter is older than the one running. **Update** opens the install page.

## Boot entry

**Key:** starter-home.boot-entry

This card is shown on Linux with no stick attached, while this computer's boot list holds a Faraday stick entry. **Remove the Faraday boot entry** removes it.
