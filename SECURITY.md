# Security Policy

## Supported versions

Only the latest release receives fixes. Update to the newest version first.

## Reporting a vulnerability

Do not open a public issue for a security problem. Report it privately through GitHub Security Advisories:

<https://github.com/lucasbrito-wdt/attack-shark-configurator/security/advisories/new>

Include the affected version and platform, steps to reproduce, and the impact you see. This is a volunteer project:
expect an acknowledgement within a few days and a fix or a decision as soon as practical. Please allow time for a fix
before disclosing publicly; you will be credited in the release notes unless you prefer otherwise.

## Scope

In scope: the app, its update mechanism (signed manifest and artifacts), and the packaging scripts. Out of scope:
vulnerabilities in the mouse firmware or the vendor's own software, which should go to the vendor.

Never share the updater signing key. If you find it exposed anywhere, report it through the same channel.
