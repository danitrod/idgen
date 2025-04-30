# keyclip

![keyclip icon](./icons/128x128.png)

Do you need to generate quick UUIDs from time to time? This is the tool for you :)

**keyclip** is a simple menu bar tool that adds a unique identifier to your clipboard with a single
keypress.

Press `Cmd+Shift+K` anywhere to instantly copy a UUID to your clipboard.

## Installation

### macOS

Download the latest version from the [releases page](https://github.com/danitrod/keyclip/releases).
Unzip the file and move the `keyclip.app` to your Applications folder.

As the app is not signed, you're going to need to allow it to run in your system preferences. After
attempting to open the app, go to `System Preferences > Privacy & Security`, scroll down and click
`Open Anyway` next to the `keyclip` app. After doing it once, you should be able to open it
normally the next time.

### Others

There is no support for other platforms at the moment, but let me know if there's interest by
opening an issue in this repo. It should be relatively easy to port it to other platforms as the app
is built with [Tauri](https://tauri.app/).
