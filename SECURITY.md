# Security

The process for reports and audits is
[`docs/AUDIT-SUBMISSIONS.md`](docs/AUDIT-SUBMISSIONS.md). Published audits
are on the Audits page: <https://faradayoffline.app/audits/>.

## Where a finding goes

All submissions go through GitHub, none by email. A pseudonymous GitHub
account is enough.

Critical and high findings go to GitHub's private vulnerability reporting
on this repository:
<https://github.com/bitcoinpasada/faraday/security/advisories/new>. A
report is seen only by the repository's owner and the reporter.

- **Critical:** a key, seed or passphrase can be extracted; the device
  signs, or shows as approved, something the user did not approve;
  Faraday's output is wrong in a way that loses funds (an address it
  cannot spend from, a backup or shares that do not restore); or
  crafted input runs code on Faraday.
- **High:** the same, but only under specific conditions (physical
  access, a non-default setting, a rare input); a vault slot opens
  without its passphrase or reveals how many slots are used; the air
  gap fails; secrets outlive the session; a tampered image or upgrade
  is accepted; Faraday Starter writes to the wrong disk; the design
  makes a mistake that loses funds or exposes a key likely, through a
  missing warning or confirmation or a misleading label.

Everything else (medium, low and notes) and every audit goes in a public
issue with the audit form:
<https://github.com/bitcoinpasada/faraday/issues/new?template=audit.yml>.
An audit with critical or high findings puts them in `private.md`, sends
it as a private report with the issue's number, and gives its hash in the
issue. The steps are in
[§3](docs/AUDIT-SUBMISSIONS.md#3-submitting), the severities in
[§2](docs/AUDIT-SUBMISSIONS.md#2-the-request).

## Faraday's response

- **Within 15 days of the issue:** a reply on it that quotes back the
  hash, if there is one, and gives a verdict on each finding: confirmed
  and fixed (with the fixing commit), confirmed and open, could not
  reproduce, or not a bug.
- **A private report with no issue:** a reply on it within 15 days of
  its arrival.
- **Then:** the audit's entry is published.
- **Private findings:** fixed, then `private.md` is published and the
  entry updated. `private.md` is published no later than 90 days after
  the private report arrives, fixed or not.
